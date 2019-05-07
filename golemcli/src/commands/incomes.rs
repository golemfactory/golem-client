use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::pay::{AsGolemPay, Income};
use structopt::{clap::arg_enum, StructOpt};

#[derive(StructOpt, Debug)]
pub struct Section {
    filter_by: Option<crate::eth::PaymentStatus>,
    #[structopt(long = "sort")]
    sort_by: Option<String>,
}

arg_enum! {
    #[derive(Debug, Clone, Copy)]
    pub enum Column {
        Payer,
        Status,
        Value
    }
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
            .get_incomes_list()
            .from_err()
            .and_then(move |incomes| {
                let columns = vec!["payer".into(), "status".into(), "value".into()];
                let values = incomes
                    .into_iter()
                    .map(|income: Income| {
                        let payer = income.payer;
                        let status = income.status;
                        let value = crate::eth::Currency::GNT.format_decimal(&income.value);

                        serde_json::json!([payer, status, value])
                    })
                    .collect();

                Ok(ResponseTable { columns, values }.sort_by(&sort_by).into())
            })
    }
}
