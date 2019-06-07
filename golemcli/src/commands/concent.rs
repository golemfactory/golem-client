use crate::context::*;
use futures::{future, Future};
use golem_rpc_api::concent::*;
use golem_rpc_api::net::AsGolemNet;
use golem_rpc_api::pay::{AsGolemPay, DepositPayment};
use std::str::FromStr;
use structopt::{clap::arg_enum, StructOpt};

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Turns concent on
    #[structopt(name = "on")]
    On,

    /// Turns concent off
    #[structopt(name = "off")]
    Off,

    /// Shows concesnt status
    #[structopt(name = "status")]
    Status,

    /// Terms of Use
    #[structopt(name = "terms")]
    Terms(Terms),

    /// Display deposit payments
    #[structopt(name = "deposit")]
    Deposit(Deposit),
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
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            Section::On => Box::new(self.run_turn(endpoint, true)),
            Section::Off => Box::new(self.run_turn(endpoint, false)),
            Section::Status => Box::new(self.status(endpoint)),
            Section::Deposit(Deposit { command: None }) => Box::new(self.deposit_status(endpoint)),
            Section::Deposit(Deposit {
                command: Some(DepositCommands::Payments { filter_by, sort_by }),
            }) => Box::new(self.deposit_payments(endpoint, filter_by, sort_by)),
            Section::Terms(terms) => terms.run(endpoint),
        }
    }

    fn run_turn(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        on: bool,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_concent()
            .turn(on)
            .from_err()
            .and_then(move |()| CommandResponse::object(status_to_msg(on)))
    }

    fn status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_concent()
            .is_on()
            .from_err()
            .and_then(|is_on| CommandResponse::object(status_to_msg(is_on)))
    }

    fn deposit_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_pay()
            .get_deposit_balance()
            .from_err()
            .and_then(|balance_info| CommandResponse::object(balance_info))
    }

    fn deposit_payments(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        filter_by: &Option<crate::eth::PaymentStatus>,
        sort_by: &Option<String>,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let sort_by = sort_by.clone();
        let filter_by = filter_by.clone();

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

impl Terms {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            Terms::Accept => Box::new(
                endpoint
                    .as_golem_concent()
                    .accept_terms()
                    .from_err()
                    .and_then(|()| {
                        CommandResponse::object("Concent terms of use have been accepted.")
                    }),
            ),
            Terms::Show => Box::new(
                endpoint
                    .as_golem_concent()
                    .show_terms()
                    .from_err()
                    .and_then(|terms_html| {
                        let text = html2text::from_read(std::io::Cursor::new(terms_html), 78);
                        CommandResponse::object(text)
                    }),
            ),
            Terms::Status => Box::new(
                endpoint
                    .as_golem_concent()
                    .is_terms_accepted()
                    .from_err()
                    .and_then(|is_terms_accepted| CommandResponse::object(is_terms_accepted)),
            ),
        }
    }
}
