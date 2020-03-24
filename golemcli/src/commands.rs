use crate::context::{CliCtx, CommandResponse};
use failure::Fallible;
use futures::{future, prelude::*};
use golem_rpc_api::rpc::*;
use std::fmt::{self, Debug};
use structopt::*;

mod account;
mod cache;
#[cfg(feature = "concent_cli")]
mod concent;
#[cfg(feature = "debug_cli")]
mod debug;
#[cfg(feature = "concent_cli")]
mod envs;
mod incomes;
mod network;
mod payments;
mod settings;
mod status;
mod tasks;
mod terms;
#[cfg(feature = "test_task_cli")]
mod test_task;
mod wallet;

mod acl;

mod resources;

#[derive(StructOpt, Debug)]
pub enum CommandSection {
    /// Manage account
    #[structopt(name = "account")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Account(account::AccountSection),

    /// Display general status
    #[structopt(name = "status")]
    Status(status::Section),

    /// Display incomes
    #[structopt(name = "incomes")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Incomes(incomes::Section),

    /// Display payments
    #[structopt(name = "payments")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Payments(payments::Section),

    /// Manage network
    #[structopt(name = "network")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Network(network::NetworkSection),

    /// Manage settings
    #[structopt(name = "settings")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Settings(settings::Section),

    /// Manage provider resources
    #[structopt(name = "res")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Res(resources::Section),

    /// Manage tasks
    #[structopt(name = "tasks")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Tasks(tasks::Section),

    /// Manage testing tasks
    #[cfg(feature = "test_task_cli")]
    #[structopt(name = "test_task")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    TestTask(test_task::Section),

    /// Manage peer access control lists
    #[structopt(name = "acl")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Acl(acl::Section),

    /// Manage environments
    #[structopt(name = "envs")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Envs(envs::Section),

    /// Concent Service
    #[cfg(feature = "concent_cli")]
    #[structopt(name = "concent")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Concent(concent::Section),

    /// Manage disc cache
    #[structopt(name = "cache")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Cache(cache::Section),

    /// Debug RPC
    #[cfg(feature = "debug_cli")]
    #[structopt(name = "debug")]
    Debug(debug::Section),

    /// Show and accept terms of use
    #[structopt(name = "terms")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Terms(terms::Section),

    /// Wallet operations
    #[structopt(name = "wallet")]
    #[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
    Wallet(wallet::Section),

    /// Quit after finishing ongoing tasks
    #[structopt(name = "shutdown")]
    Shutdown(ShutdownCommand),

    #[structopt(name = "_int")]
    #[structopt(raw(setting = "structopt::clap::AppSettings::Hidden"))]
    Internal(InternalSection),
}

macro_rules! dispatch_subcommand {
    {
        on ($self:expr, $ctx:expr);
        $(async {
            $(
                $(#[$async_meta:meta])*
                $async_command:path
            ,)*
        })?
        $(async_with_ctx {
            $(
            $(#[$async_with_context_meta:meta])*
            $async_with_context_command:path,)*
        })?
        $(sync {
            $($sync_command:path),*
        })?
    } => {{
        match $self {
            $(
                $(
                      $(#[$async_meta])*
                      $async_command(command) => {
                         let endpoint = $ctx.connect_to_app().await?;
                         let endpoint = $ctx.unlock_app(endpoint).await?;
                         command.run(endpoint).await
                      }
                )*
            )?,
            $(
                $(
                    $sync_command(command) => command.run_command()
                ),*
            )?,
            $(
                $(
                    $(#[$async_with_context_meta])*
                    $async_with_context_command(command) => {
                         let endpoint = $ctx.connect_to_app().await?;
                         command.run($ctx, endpoint).await
                    }
                )*
            )?
        }
    }};
}

impl CommandSection {
    pub async fn run_command(&self, ctx: &mut CliCtx) -> Fallible<CommandResponse> {
        dispatch_subcommand! {
            on (self, ctx);
            async {
                #[cfg(feature = "concent_cli")]
                CommandSection::Concent,
                CommandSection::Network,
                CommandSection::Envs,
                CommandSection::Incomes,
                CommandSection::Payments,
                CommandSection::Wallet,
                CommandSection::Cache,
                CommandSection::Settings,
                CommandSection::Tasks,
                #[cfg(feature = "test_task_cli")]
                CommandSection::TestTask,
                CommandSection::Shutdown,
                CommandSection::Res,
            }
            async_with_ctx {
                CommandSection::Account,
                CommandSection::Acl,
                CommandSection::Terms,
                CommandSection::Status,
                #[cfg(feature = "debug_cli")]
                CommandSection::Debug,
            }
            sync {
                CommandSection::Internal
            }
        }
    }
}

#[derive(StructOpt)]
pub enum InternalSection {
    /// Generates autocomplete script from given shell
    #[structopt(name = "complete")]
    Complete {
        /// Describes which shell to produce a completions file for
        #[structopt(
            parse(try_from_str),
            raw(
                possible_values = "&clap::Shell::variants()",
                case_insensitive = "true"
            )
        )]
        shell: clap::Shell,
    },
}

impl Debug for InternalSection {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            InternalSection::Complete { shell } => writeln!(f, "complete({})", shell),
        }
    }
}

impl InternalSection {
    fn run_command(&self) -> Fallible<CommandResponse> {
        match self {
            InternalSection::Complete { shell } => super::CliArgs::clap().gen_completions_to(
                "golemcli",
                *shell,
                &mut std::io::stdout(),
            ),
        }

        Ok(CommandResponse::NoOutput)
    }
}

#[derive(StructOpt, Debug)]
pub struct ShutdownCommand {}

impl ShutdownCommand {
    async fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        let ret: serde_json::Value = endpoint
            .as_invoker()
            .rpc_call("golem.graceful_shutdown", &())
            .await?;

        let result = format!("Graceful shutdown triggered result: {}", ret);
        Ok(CommandResponse::Object(result.into()))
    }
}
