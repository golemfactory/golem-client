use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::pay::{AsGolemPay, DepositPayment};
use structopt::StructOpt;

const DEPOSIT_COLUMNS: &[&str] = &["tx", "status", "value", "fee"];

#[derive(StructOpt, Debug)]
pub struct Section {
    /// Filter by status
    #[structopt(
        parse(try_from_str),
        raw(
            possible_values = "&[\"awaiting\",\"confirmed\"]",
            case_insensitive = "true"
        )
    )]
    filter_by: Option<crate::eth::PaymentStatus>,
    #[structopt(long = "sort")]
    #[structopt(
        parse(try_from_str),
        raw(possible_values = "DEPOSIT_COLUMNS", case_insensitive = "true")
    )]
    /// Sort incomes
    sort_by: Option<String>,
}

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let sort_by = self.sort_by.clone();
        let filter_by = self.filter_by.clone();

        endpoint
            .as_golem_pay()
            .get_deposit_payments_list(None, None)
            .from_err()
            .and_then(move |payments| {
                let columns = DEPOSIT_COLUMNS.iter().map(|&name| name.into()).collect();
                let values = payments
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|payment| {
                        filter_by
                            .map(|f| f.is_match_with(&payment.status))
                            .unwrap_or(true)
                    })
                    .map(|payment: DepositPayment| {
                        let value = crate::eth::Currency::GNT.format_decimal(&payment.value);
                        let fee = payment
                            .fee
                            .map(|fee| crate::eth::Currency::ETH.format_decimal(&fee));

                        serde_json::json!([payment.transaction, payment.status, value, fee])
                    })
                    .collect();

                Ok(ResponseTable { columns, values }.sort_by(&sort_by).into())
            })
    }
}
