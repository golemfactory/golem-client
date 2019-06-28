use crate::context::*;
use crate::eth::Currency;
use bigdecimal::BigDecimal;
use failure::Fallible;
use futures::{future, prelude::*};
use golem_rpc_api::core::AsGolemCore;
use golem_rpc_api::net::AsGolemNet;
use golem_rpc_api::pay::{Balance, AsGolemPay};
use golem_rpc_api::rpc::*;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum AccountSection {
    /// Display account & financial info
    #[structopt(name = "info")]
    Info,
    /// Trigger graceful shutdown of your golem
    #[structopt(name = "shutdown")]
    Shutdown,
    /// Unlock account, will prompt for your password
    #[structopt(name = "unlock")]
    Unlock,

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
}

impl AccountSection {
    pub fn run(
        &self,
        ctx: &CliCtx,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            AccountSection::Unlock => Box::new(self.account_unlock(endpoint)),
            AccountSection::Info => Box::new(self.account_info(endpoint)),
            AccountSection::Shutdown => Box::new(self.account_shutdown(endpoint)),
            AccountSection::Withdraw {
                destination,
                amount,
                currency,
                gas_price,
            } => Box::new(self.withdraw(ctx, destination, amount, currency, gas_price, endpoint)),
        }
    }

    fn account_unlock(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem()
            .is_account_unlocked()
            .and_then(move |unlocked| {
                if unlocked {
                    return future::Either::B(future::ok(CommandResponse::Object(
                        serde_json::json!("Account already unlocked"),
                    )));
                }

                future::Either::A(
                    rpassword::read_password_from_tty(Some(
                        "Unlock your account to start golem\n\
                         This command will time out in 30 seconds.\n\
                         Password: ",
                    ))
                    .into_future()
                    .from_err()
                    .and_then(move |password| endpoint.as_golem().set_password(password))
                    .and_then(|result| {
                        if result {
                            Ok(CommandResponse::Object(serde_json::json!(
                                "Account unlock success"
                            )))
                        } else {
                            Ok(CommandResponse::Object(serde_json::json!(
                                "Incorrect password"
                            )))
                        }
                    }),
                )
            })
            .from_err()
    }

    fn account_info(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint.as_golem_net().get_node().and_then(
            move |node: golem_rpc_api::net::NodeInfo| {
                let query = {
                    let node_id = node.key.to_string();
                    let computing_trust = endpoint.as_invoker().rpc_call("rep.comp", &(node_id.clone(), ));
                    let requesting_trust = endpoint.as_invoker().rpc_call("rep.requesting", &(node_id, ));
                    let payment_address = endpoint.as_golem_pay().get_pay_ident().from_err();
                    let balance = endpoint.as_golem_pay().get_pay_balance().from_err();
                    let deposit_balance = endpoint.as_invoker().rpc_call("pay.deposit_balance", &());

                    computing_trust
                        .join5(requesting_trust, payment_address, balance, deposit_balance)
                };

                // TODO: deposit_balance formating
                query
                    .and_then(
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

                            Ok(CommandResponse::Object(serde_json::json!({
                                "node_name": node.node_name,
                                "Golem_ID": node.key,
                                "requestor_reputation": (requesting_trust.unwrap_or_default()*100.0) as u64,
                                "provider_reputation": (computing_trust.unwrap_or_default()*100.0) as u64,
                                "finances": {
                                    "eth_address": payment_address,
                                    "eth_available": eth_available,
                                    "eth_locked": eth_locked,
                                    "gnt_available": gnt_available,
                                    "gnt_locked": balance.gnt_lock,
                                    "gnt_unadopted": gnt_unadopted,
                                    "deposit_balance": deposit_balance
                                }
                            })))

                        },
                    )
            },
        ).from_err()
    }

    fn account_shutdown(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_invoker()
            .rpc_call("golem.graceful_shutdown", &())
            .and_then(|ret: u64| {
                let result = format!("Graceful shutdown triggered result: {}", ret);
                Ok(CommandResponse::Object(result.into()))
            })
            .from_err()
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
                        amount.clone(),
                        destination.clone(),
                        currency.clone(),
                        gas_price.clone(),
                    ),
                )
                .from_err()
                .and_then(|transactions: Vec<String>| CommandResponse::object(transactions)),
        )
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum DepositStatus {
    Locked,
    Unlocking,
    Unlocked,
}

#[derive(Deserialize, Serialize)]
struct DepositBalance {
    value: String,
    status: DepositStatus,
    timelock: String,
}
