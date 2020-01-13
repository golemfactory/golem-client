use crate::context::*;
use bigdecimal::BigDecimal;
use futures::prelude::*;
use golem_rpc_api::pay::{AsGolemPay, Payment, PaymentStatus};
use std::collections::{BTreeMap, HashMap};
use structopt::StructOpt;

const PAYMENTS_COLUMNS: &[&str] = &["subtask", "payee", "status", "value", "fee"];

#[derive(StructOpt, Debug)]
pub struct Section {
    /// Sort payments
    #[structopt(long = "sort")]
    #[structopt(
        parse(try_from_str),
        raw(possible_values = "PAYMENTS_COLUMNS", case_insensitive = "true")
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
    /// Display all payments
    #[structopt(name = "all")]
    All,
    /// Display awaiting payments
    #[structopt(name = "awaiting")]
    Awaiting,
    /// Display confirmed payments
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

impl Section {
    pub async fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> failure::Fallible<CommandResponse> {
        self.show(
            endpoint,
            &self.command.to_payment_status(),
            &self.sort_by,
            &self.full,
        )
        .await
    }

    async fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        filter_by: &Option<crate::eth::PaymentStatus>,
        sort_by: &Option<String>,
        full: &bool,
    ) -> failure::Fallible<CommandResponse> {
        let sort_by = sort_by.clone();
        let filter_by = filter_by.clone();
        let full = *full;

        let payments: Vec<Payment> = endpoint
            .as_golem_pay()
            .get_payments_list(None, None)
            .await?;
        let columns = PAYMENTS_COLUMNS.iter().map(|&name| name.into()).collect();
        let mut total_value = BigDecimal::from(0u32);
        let mut total_fee = BigDecimal::from(0u32);
        let mut total_for_status: BTreeMap<PaymentStatus, BigDecimal> = BTreeMap::new();
        let mut fee_for_status: BTreeMap<PaymentStatus, BigDecimal> = BTreeMap::new();
        let values = payments
            .into_iter()
            .filter(|payment| {
                filter_by
                    .map(|f| f.is_match_with(&payment.status))
                    .unwrap_or(true)
            })
            .map(|payment: Payment| {
                total_value += &payment.value;

                if let Some(total) = total_for_status.get_mut(&payment.status) {
                    *total = total.clone() + payment.value.clone();
                } else {
                    total_for_status.insert(payment.status.clone(), payment.value.clone());
                }

                if let Some(fee) = &payment.fee {
                    total_fee += fee;
                    if let Some(status_fee) = fee_for_status.get_mut(&payment.status) {
                        *status_fee = status_fee.clone() + fee;
                    } else {
                        fee_for_status.insert(payment.status.clone(), fee.clone());
                    }
                }

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

        let mut summary = Vec::new();
        if total_for_status.len() > 1 {
            for (k, v) in total_for_status {
                let fee = fee_for_status.get(&k);
                summary.push(serde_json::json!([
                    "",
                    "",
                    k,
                    crate::eth::Currency::GNT.format_decimal(&v),
                    fee.map(|fee| crate::eth::Currency::ETH.format_decimal(&fee))
                ]));
            }
        }
        summary.push(serde_json::json!([
            "",
            "",
            "total",
            crate::eth::Currency::GNT.format_decimal(&total_value),
            crate::eth::Currency::ETH.format_decimal(&total_fee)
        ]));

        Ok(ResponseTable { columns, values }
            .sort_by(&sort_by)
            .with_summary(summary))
    }
}
