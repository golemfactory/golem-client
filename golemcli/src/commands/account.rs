use crate::context::*;
use crate::eth::Currency;
use crate::formaters::*;
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
    pub async fn run(
        &self,
        ctx: &mut CliCtx,
        endpoint: impl actix_wamp::RpcEndpoint + actix_wamp::PubSubEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        match self {
            AccountSection::Unlock => self.account_unlock(endpoint).await,
            AccountSection::Info => {
                let endpoint = ctx.unlock_app(endpoint).await?;
                account_info(endpoint).await
            }
            AccountSection::Withdraw {
                destination,
                amount,
                currency,
                gas_price,
            } => {
                self.withdraw(ctx, destination, amount, currency, gas_price, endpoint)
                    .await
            }
        }
    }

    async fn account_unlock(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        let unlocked = endpoint.as_golem().is_account_unlocked().await?;

        if unlocked {
            CommandResponse::object("Account already unlocked")
        } else {
            crate::account::account_unlock(endpoint).await?;
            CommandResponse::object("Account unlock success")
        }
    }

    async fn withdraw(
        &self,
        ctx: &CliCtx,
        destination: &String,
        amount: &bigdecimal::BigDecimal,
        currency: &crate::eth::Currency,
        gas_price: &Option<bigdecimal::BigDecimal>,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        use crate::eth::Currency::ETH;
        let ack = ctx.prompt_for_acceptance("Are you sure?", None, Some("Withdraw cancelled"));

        if !ack {
            return Ok(CommandResponse::NoOutput);
        }

        let transactions = endpoint
            .as_pay()
            .withdraw(
                currency.from_user(amount),
                destination.clone(),
                currency.to_string(),
                gas_price,
            )
            .await?;
        CommandResponse::object(transactions)
    }
}

#[derive(Serialize)]
struct AccountInfo {
    #[serde(rename = "Golem_ID")]
    golem_id: String,
    node_name: String,
    requestor_reputation: u64,
    provider_reputation: u64,
    finances: serde_json::Value,
}

async fn account_info(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
) -> Fallible<CommandResponse> {
    let node_info = endpoint.as_golem_net().get_node().await?;

    let node_id = node_info.key.to_string();
    let (computing_trust, requesting_trust, payment_address, balance, deposit_balance): (
        Option<f64>,
        Option<f64>,
        _,
        _,
        _,
    ) = future::try_join5(
        endpoint
            .as_invoker()
            .rpc_call("rep.comp", &(node_id.clone(),)),
        endpoint
            .as_invoker()
            .rpc_call("rep.requesting", &(node_id,)),
        endpoint.as_golem_pay().get_pay_ident(),
        endpoint.as_golem_pay().get_pay_balance(),
        endpoint.as_golem_pay().get_deposit_balance(),
    )
    .await?;

    let eth_available = Currency::ETH.format_decimal(&balance.eth);
    let eth_locked = Currency::ETH.format_decimal(&balance.eth_lock);
    let gnt_available = Currency::GNT.format_decimal(&balance.av_gnt);
    let gnt_unadopted = Currency::GNT.format_decimal(&balance.gnt_nonconverted);
    let gnt_locked = Currency::GNT.format_decimal(&balance.gnt_lock);

    CommandResponse::object(AccountInfo {
        node_name: node_info.node_name,
        golem_id: node_info.key,
        requestor_reputation: (requesting_trust.unwrap_or_default() * 100.0) as u64,
        provider_reputation: (computing_trust.unwrap_or_default() * 100.0) as u64,
        finances: serde_json::json!({
            "eth_address": payment_address,
            "eth_available": eth_available,
            "eth_locked": eth_locked,
            "gnt_available": gnt_available,
            "gnt_locked": gnt_locked,
            "gnt_unadopted": gnt_unadopted,
            "deposit_balance": deposit_balance.map(|b : golem_rpc_api::pay::DepositBalance| b.humanize())
        }),
    })
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum DepositStatus {
    Locked,
    Unlocking,
    Unlocked,
}
