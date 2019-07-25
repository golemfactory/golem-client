use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::pay::{AsGolemPay, Payment};
use structopt::StructOpt;

const PAYMENTS_COLUMNS: &[&str] = &["subtask", "payee", "status", "value", "fee"];

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Show payments
    #[structopt(name = "show")]
    Show {
        /// Filter by status
        #[structopt(
            parse(try_from_str),
            raw(
                possible_values = "&[\"awaiting\",\"confirmed\"]",
                case_insensitive = "true"
            )
        )]
        filter_by: Option<crate::eth::PaymentStatus>,
        /// Sort payments
        #[structopt(long = "sort")]
        #[structopt(
            parse(try_from_str),
            raw(possible_values = "PAYMENTS_COLUMNS", case_insensitive = "true")
        )]
        sort_by: Option<String>,
        #[structopt(long)]
        full: bool,
    },
}

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            Section::Show {
                filter_by,
                sort_by,
                full,
            } => Box::new(self.show(endpoint, filter_by, sort_by, full)),
        }
    }

    pub fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        filter_by: &Option<crate::eth::PaymentStatus>,
        sort_by: &Option<String>,
        full: &bool,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let sort_by = sort_by.clone();
        let filter_by = filter_by.clone();
        let full = *full;

        endpoint
            .as_golem_pay()
            .get_payments_list(None, None)
            .from_err()
            .and_then(move |payments: Vec<Payment>| {
                let columns = PAYMENTS_COLUMNS.iter().map(|&name| name.into()).collect();
                let values = payments
                    .into_iter()
                    .filter(|payment| {
                        filter_by
                            .map(|f| f.is_match_with(&payment.status))
                            .unwrap_or(true)
                    })
                    .map(|payment: Payment| {
                        let subtask = payment.subtask;
                        let payer = if full || payment.payee.len() == 42 {
                            payment.payee
                        } else {
                            crate::eth::public_to_addres(payment.payee)
                        };
                        let status = payment.status;
                        let value = crate::eth::Currency::GNT.format_decimal(&payment.value);
                        let fee = payment
                            .fee
                            .map(|fee| crate::eth::Currency::ETH.format_decimal(&fee));

                        serde_json::json!([subtask, payer, status, value, fee])
                    })
                    .collect();
                Ok(ResponseTable { columns, values }.sort_by(&sort_by).into())
            })
    }
}
