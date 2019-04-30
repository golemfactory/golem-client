use crate::context::CliCtx;
use crate::rpc::*;
use failure::Fallible;
use futures::future::Future;
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
    pub fn run(&self, ctx: &mut CliCtx) -> Fallible<()> {
        Ok(match self {
            AccountSection::Unlock => self.account_unlock(ctx)?,
            AccountSection::Info => self.account_info(ctx)?,
            AccountSection::Shutdown => self.account_shutdown(ctx)?,
            AccountSection::Withdraw {
                destination,
                amount,
                currency,
                gas_price,
            } => self.withdraw(destination, amount, currency, gas_price, ctx)?,
        })
    }

    fn account_unlock(&self, ctx: &mut CliCtx) -> Fallible<()> {
        let (mut sys, rpc) = ctx.connect_to_app()?;

        let unlocked: bool =
            sys.block_on(rpc.as_invoker().rpc_call("golem.password.unlocked", ()))?;

        if unlocked {
            ctx.message("Account already unlocked");
        } else {
            let password = rpassword::read_password_from_tty(Some(
                "Unlock your account to start golem\n\
                 This command will time out in 30 seconds.\n\
                 Password: ",
            ))?;

            if sys.block_on(rpc.as_invoker().rpc_call("golem.password.set", (password,)))? {
                ctx.message("Account unlock success");
            } else {
                ctx.message("Incorrect password");
            }
        }
        Ok(())
    }

    fn account_info(&self, ctx: &mut CliCtx) -> Fallible<()> {
        let (mut sys, rpc) = ctx.connect_to_app()?;

        let json : serde_json::Value = sys.block_on(rpc.as_invoker().rpc_call("net.ident", ()).and_then(
            |node: golem_rpc_api::net::NodeInfo| {
                let node_id = node.key.clone();

                let query = {

                    let computing_trust = rpc.as_invoker().rpc_call("rep.comp", (node_id.clone(), ));
                    let requesting_trust = rpc.as_invoker().rpc_call("rep.requesting", (node_id.clone(), ));
                    let payment_address = rpc.as_invoker().rpc_call("pay.ident", ()); // Option<String>
                    let balance = rpc.as_invoker().rpc_call("pay.balance", ());
                    let deposit_balance = rpc.as_invoker().rpc_call("pay.deposit_balance", ());

                    computing_trust
                        .join5(requesting_trust, payment_address, balance, deposit_balance)
                };

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
                            serde_json::Value,
                            Option<DepositBalance>,
                        )| {
                            Ok(serde_json::json!({
                                "node_name": node.node_name,
                                "Golem_ID": node.key,
                                "requestor_reputation": (requesting_trust.unwrap_or_default()*100.0) as u64,
                                "provider_reputation": (computing_trust.unwrap_or_default()*100.0) as u64,
                                "finances": {
                                    "eth_address": payment_address,
                                    "eth_available": balance["eth"],
                                    "eth_locked": balance["eth_lock"],
                                    "gnt_available": balance["av_gnt"],
                                    "gnt_locked": balance["gnt_lock"],
                                    "gnt_unadopted": balance["gnt_nonconverted"],
                                    "deposit_balance": deposit_balance
                                }
                            }))
                        },
                    )
            },
        ))?;

        ctx.message(&format!("{:#}", json));

        Ok(())
    }

    fn account_shutdown(&self, ctx: &mut CliCtx) -> Fallible<()> {
        let (mut sys, rpc) = ctx.connect_to_app()?;

        let ret: u64 = sys.block_on(rpc.as_invoker().rpc_call("golem.graceful_shutdown", ()))?;
        ctx.message(&format!("Graceful shutdown triggered result: {}", ret));
        // "" -> quit=0, off=1, on=1
        Ok(())
    }

    fn withdraw(
        &self,
        destination: &String,
        amount: &bigdecimal::BigDecimal,
        currency: &crate::eth::Currency,
        gas_price: &Option<bigdecimal::BigDecimal>,
        ctx: &mut CliCtx,
    ) -> Fallible<()> {
        let (mut sys, rpc) = ctx.connect_to_app()?;

        let transactions: Vec<String> = sys.block_on(
            rpc.as_invoker()
                .rpc_call("pay.withdraw", (amount, destination, currency)),
        )?;

        ctx.message(&format!("{:?}", transactions));

        Ok(())
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
