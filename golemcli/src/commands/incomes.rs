use crate::context::*;
use bigdecimal::BigDecimal;
use futures::prelude::*;
use golem_rpc_api::pay::{AsGolemPay, Income, PaymentStatus};
use std::collections::BTreeMap;
use structopt::{clap::arg_enum, StructOpt};

const INCOMES_COLUMNS: &[&str] = &["payer", "status", "value"];

#[derive(StructOpt, Debug)]
pub struct Section {
    /// Sort payments
    #[structopt(long = "sort")]
    #[structopt(
        parse(try_from_str),
        raw(possible_values = "INCOMES_COLUMNS", case_insensitive = "true")
    )]
    sort_by: Option<String>,
    #[structopt(long)]
    #[structopt(raw(set = "structopt::clap::ArgSettings::Hidden"))]
    #[structopt(raw(set = "structopt::clap::ArgSettings::Global"))]
    full: bool,

    #[structopt(subcommand)]
    command: SectionCommand,
}

#[derive(StructOpt, Debug)]
enum SectionCommand {
    /// Display all incomes
    #[structopt(name = "all")]
    All,
    /// Display awaiting incomes
    #[structopt(name = "awaiting")]
    Awaiting,
    /// Display confirmed incomes
    #[structopt(name = "confirmed")]
    Confirmed,
}

impl SectionCommand {
    fn to_payment_status(&self) -> Option<crate::eth::PaymentStatus> {
        match self {
            SectionCommand::All => None,
            SectionCommand::Awaiting => Some(crate::eth::PaymentStatus::Awaiting),
            SectionCommand::Confirmed => Some(crate::eth::PaymentStatus::Confirmed),
        }
    }
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
        Box::new(self.show(
            endpoint,
            &self.command.to_payment_status(),
            &self.sort_by,
            &self.full,
        ))
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
                let mut total_value = BigDecimal::from(0u32);
                let mut total_for_status: BTreeMap<PaymentStatus, BigDecimal> = BTreeMap::new();

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

                        total_value += &income.value;
                        if let Some(total) = total_for_status.get_mut(&income.status) {
                            *total = total.clone() + income.value.clone();
                        } else {
                            total_for_status.insert(income.status.clone(), income.value.clone());
                        }

                        let status = income.status;
                        let value = crate::eth::Currency::GNT.format_decimal(&income.value);

                        serde_json::json!([payer, status, value])
                    })
                    .collect();

                let mut summary = Vec::new();
                if total_for_status.len() > 1 {
                    for (k, v) in total_for_status {
                        summary.push(serde_json::json!([
                            "",
                            k,
                            crate::eth::Currency::GNT.format_decimal(&v)
                        ]));
                    }
                }
                summary.push(serde_json::json!([
                    "",
                    "total",
                    crate::eth::Currency::GNT.format_decimal(&total_value)
                ]));

                Ok(ResponseTable { columns, values }
                    .sort_by(&sort_by)
                    .with_summary(summary))
            })
    }
}
