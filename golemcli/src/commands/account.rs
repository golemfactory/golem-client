use crate::context::*;
use crate::eth::Currency;
use actix::fut::Either;
use bigdecimal::BigDecimal;
use failure::Fallible;
use futures::{future, prelude::*};
use golem_rpc_api::core::AsGolemCore;
use golem_rpc_api::net::AsGolemNet;
use golem_rpc_api::pay::{AsGolemPay, Balance, DepositBalance};
use golem_rpc_api::rpc::*;
use serde::{Deserialize, Serialize};
use structopt::{clap, StructOpt};
use crate::formaters::*;

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
pub enum AccountSection {
    /// Display account & financial info
    #[structopt(name = "info")]
    Info,

    /// Withdraw GNT/ETH (withdrawals are not available for the testnet)
    #[structopt(name = "withdraw")]
    Withdraw {
        /// Address to send the funds to
        destination: String,
        /// Amount to withdraw, eg 1.45
        amount: bigdecimal::BigDecimal,
        /// ETH or GNT
        currency: crate::eth::Currency,
        /// Gas price in wei (not gwei)
        gas_price: Option<bigdecimal::BigDecimal>,
    },
    /// Unlock account, will prompt for your password
    #[structopt(name = "unlock")]
    Unlock,
}

impl AccountSection {
    pub fn run(
        &self,
        ctx: &mut CliCtx,
        endpoint: impl actix_wamp::RpcEndpoint + actix_wamp::PubSubEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let x = || -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
            match self {
                AccountSection::Unlock => Box::new(self.account_unlock(endpoint)),
                AccountSection::Info => Box::new(
                    ctx.unlock_app(endpoint)
                        .into_future()
                        .and_then(|endpoint| account_info(endpoint)),
                ),
                AccountSection::Withdraw {
                    destination,
                    amount,
                    currency,
                    gas_price,
                } => {
                    Box::new(self.withdraw(ctx, destination, amount, currency, gas_price, endpoint))
                }
            }
        };

        x()
    }

    fn account_unlock(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem()
            .is_account_unlocked()
            .from_err()
            .and_then(move |unlocked| {
                if unlocked {
                    future::Either::A(
                        CommandResponse::object("Account already unlocked").into_future(),
                    )
                } else {
                    future::Either::B(
                        crate::account::account_unlock(endpoint)
                            .and_then(|()| CommandResponse::object("Account unlock success")),
                    )
                }
            })
    }

    fn withdraw(
        &self,
        ctx: &CliCtx,
        destination: &String,
        amount: &bigdecimal::BigDecimal,
        currency: &crate::eth::Currency,
        gas_price: &Option<bigdecimal::BigDecimal>,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        use crate::eth::Currency::ETH;
        let ack = ctx.prompt_for_acceptance("Are you sure?", None, Some("Withdraw cancelled"));

        if !ack {
            return future::Either::A(future::ok(CommandResponse::NoOutput));
        }
        future::Either::B(
            endpoint
                .as_invoker()
                .rpc_call(
                    "pay.withdraw",
                    &(
                        currency.from_user(amount),
                        destination.clone(),
                        currency.clone(),
                        gas_price
                            .as_ref()
                            .map(|gas_price| ETH.from_user(&gas_price)),
                    ),
                )
                .from_err()
                .and_then(|transactions: Vec<String>| CommandResponse::object(transactions)),
        )
    }
}

#[derive(Serialize)]
struct AccountInfo {
    #[serde(rename="Golem_ID")]
    golem_id : String,
    node_name : String,
    requestor_reputation : u64,
    provider_reputation : u64,
    finances : serde_json::Value
}

fn account_info(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
    endpoint
        .as_golem_net()
        .get_node()
        .from_err()
        .and_then(move |node: golem_rpc_api::net::NodeInfo| {
            let query = {
                let node_id = node.key.to_string();
                let computing_trust = endpoint
                    .as_invoker()
                    .rpc_call("rep.comp", &(node_id.clone(),));
                let requesting_trust = endpoint
                    .as_invoker()
                    .rpc_call("rep.requesting", &(node_id,));
                let payment_address = endpoint.as_golem_pay().get_pay_ident().from_err();
                let balance = endpoint.as_golem_pay().get_pay_balance().from_err();
                let deposit_balance = endpoint.as_golem_pay().get_deposit_balance();

                computing_trust.join5(requesting_trust, payment_address, balance, deposit_balance)
            };

            // TODO: deposit_balance formating
            query.from_err().and_then(
                move |(
                    computing_trust,
                    requesting_trust,
                    payment_address,
                    balance,
                    deposit_balance,
                ): (
                    Option<f64>,
                    Option<f64>,
                    String,
                    Balance,
                    Option<DepositBalance>,
                )| {
                    let eth_available = Currency::ETH.format_decimal(&balance.eth);
                    let eth_locked = Currency::ETH.format_decimal(&balance.eth_lock);
                    let gnt_available = Currency::GNT.format_decimal(&balance.av_gnt);
                    let gnt_unadopted = Currency::GNT.format_decimal(&balance.gnt_nonconverted);
                    let gnt_locked = Currency::GNT.format_decimal(&balance.gnt_lock);

                    CommandResponse::object(AccountInfo {
                        node_name: node.node_name,
                        golem_id: node.key,
                        requestor_reputation: (requesting_trust.unwrap_or_default()*100.0) as u64,
                        provider_reputation: (computing_trust.unwrap_or_default()*100.0) as u64,
                        finances: serde_json::json!({
                            "eth_address": payment_address,
                            "eth_available": eth_available,
                            "eth_locked": eth_locked,
                            "gnt_available": gnt_available,
                            "gnt_locked": gnt_locked,
                            "gnt_unadopted": gnt_unadopted,
                            "deposit_balance": deposit_balance.map(|b : golem_rpc_api::pay::DepositBalance| b.humanize())
                        })
                    })
                },
            )
        })
        .from_err()
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum DepositStatus {
    Locked,
    Unlocking,
    Unlocked,
}

