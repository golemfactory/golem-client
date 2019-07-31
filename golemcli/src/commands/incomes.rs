use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::pay::{AsGolemPay, Income};
use structopt::{clap::arg_enum, StructOpt};

const INCOMES_COLUMNS: &[&str] = &["payer", "status", "value"];

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Show incomes
    #[structopt(name = "show")]
    Show {
        /// Filter by status
        #[structopt(long = "filter")]
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
            raw(possible_values = "INCOMES_COLUMNS", case_insensitive = "true")
        )]
        /// Sort incomes
        sort_by: Option<String>,

        /// Show full table contents
        #[structopt(long)]
        full: bool,
    },
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
        let filter_by = filter_by.clone();
        let sort_by = sort_by.clone();
        let full = *full;

        endpoint
            .as_golem_pay()
            .get_incomes_list()
            .from_err()
            .and_then(move |incomes| {
                let columns = INCOMES_COLUMNS.iter().map(|&name| name.into()).collect();
                let values = incomes
                    .into_iter()
                    .filter(|income| {
                        filter_by
                            .map(|f| f.is_match_with(&income.status))
                            .unwrap_or(true)
                    })
                    .map(|income: Income| {
                        let payer = match full {
                            false => crate::eth::public_to_addres(income.payer),
                            true => income.payer,
                        };
                        let status = income.status;
                        let value = crate::eth::Currency::GNT.format_decimal(&income.value);

                        serde_json::json!([payer, status, value])
                    })
                    .collect();

                Ok(ResponseTable { columns, values }.sort_by(&sort_by).into())
            })
    }
}
