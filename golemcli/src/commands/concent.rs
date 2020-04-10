use crate::context::*;
use crate::formaters::*;
use failure::Fallible;
use futures::prelude::*;
use golem_rpc_api::concent::*;
use golem_rpc_api::net::AsGolemNet;
use golem_rpc_api::pay::{AsGolemPay, DepositPayment};
use std::str::FromStr;
use structopt::{clap::arg_enum, StructOpt};

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Shows Concent Service status
    #[structopt(name = "status")]
    Status,

    /// Turns Concent Service on
    #[structopt(name = "on")]
    On,

    /// Turns Concent Service off
    #[structopt(name = "off")]
    Off,

    /// Display deposit status
    #[structopt(name = "deposit")]
    Deposit(Deposit),

    /// Terms of Use
    #[structopt(name = "terms")]
    Terms(Terms),
}

#[derive(StructOpt, Debug)]
pub enum Terms {
    /// Shows concent terms of use.
    #[structopt(name = "show")]
    Show,
    /// Accept concent terms of use
    #[structopt(name = "accept")]
    Accept,
    /// Status is concent terms of use accepted
    #[structopt(name = "status")]
    Status,
}

#[derive(StructOpt, Debug)]
pub struct Deposit {
    #[structopt(subcommand)]
    command: Option<DepositCommands>,
}

const DEPOSIT_COLUMNS: &[&str] = &["tx", "status", "value", "fee"];

#[derive(StructOpt, Debug)]
enum DepositCommands {
    /// Display
    #[structopt(name = "payments")]
    Payments {
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
    },
}

fn status_to_msg(on: bool) -> &'static str {
    if on {
        "Concent is turned on"
    } else {
        "Concent is turned off"
    }
}

impl Section {
    pub async fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        match self {
            Section::On => self.run_turn(endpoint, true).await,
            Section::Off => self.run_turn(endpoint, false).await,
            Section::Status => self.status(endpoint).await,
            Section::Deposit(Deposit { command: None }) => self.deposit_status(endpoint).await,
            Section::Deposit(Deposit {
                command: Some(DepositCommands::Payments { filter_by, sort_by }),
            }) => self.deposit_payments(endpoint, filter_by, sort_by).await,
            Section::Terms(terms) => terms.run(endpoint).await,
        }
    }

    async fn run_turn(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        on: bool,
    ) -> Fallible<CommandResponse> {
        endpoint.as_golem_concent().turn(on).await?;
        CommandResponse::object(status_to_msg(on))
    }

    async fn status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        CommandResponse::object(status_to_msg(endpoint.as_golem_concent().is_on().await?))
    }

    async fn deposit_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        let balance_info = endpoint.as_golem_pay().get_deposit_balance().await?;
        CommandResponse::object(balance_info.map(Humanize::humanize))
    }

    async fn deposit_payments(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        filter_by: &Option<crate::eth::PaymentStatus>,
        sort_by: &Option<String>,
    ) -> Fallible<CommandResponse> {
        let sort_by = sort_by.clone();
        let filter_by = filter_by.clone();

        let payments = endpoint
            .as_golem_pay()
            .get_deposit_payments_list(None, None)
            .await?;

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
    }
}

impl Terms {
    pub async fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        match self {
            Terms::Accept => {
                endpoint.as_golem_concent().accept_terms().await?;
                CommandResponse::object("Concent terms of use have been accepted.")
            }
            Terms::Show => {
                let terms_html = endpoint.as_golem_concent().show_terms().await?;
                let text = html2text::from_read(std::io::Cursor::new(terms_html), 78);
                CommandResponse::object(text)
            }
            Terms::Status => {
                CommandResponse::object(endpoint.as_golem_concent().is_terms_accepted().await?)
            }
        }
    }
}
