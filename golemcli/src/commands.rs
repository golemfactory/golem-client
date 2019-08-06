use crate::context::{CliCtx, CommandResponse};
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

mod acl;

#[derive(StructOpt, Debug)]
pub enum CommandSection {
    /// Manage account
    #[structopt(name = "account")]
    Account(account::AccountSection),

    /// Concent Service
    #[cfg(feature = "concent_cli")]
    #[structopt(name = "concent")]
    Concent(concent::Section),

    /// Debug RPC
    #[cfg(feature = "debug_cli")]
    #[structopt(name = "debug")]
    Debug(debug::Section),

    /// Manage environments
    #[structopt(name = "envs")]
    Envs(envs::Section),

    /// Manage network
    #[structopt(name = "network")]
    Network(network::NetworkSection),

    /// Manage peer access control lists
    #[structopt(name = "acl")]
    Acl(acl::Section),

    /// Display incomes
    #[structopt(name = "incomes")]
    Incomes(incomes::Section),

    /// Display payments
    #[structopt(name = "payments")]
    Payments(payments::Section),

    /// Manage resources
    #[structopt(name = "cache")]
    Cache(cache::Section),

    /// Manage settings
    #[structopt(name = "settings")]
    Settings(settings::Section),

    /// Manage tasks
    #[structopt(name = "tasks")]
    Tasks(tasks::Section),

    /// Display general status
    #[structopt(name = "status")]
    Status(status::Section),

    /// Show and accept terms of use
    #[structopt(name = "terms")]
    Terms(terms::Section),

    /// Manage testing tasks
    #[cfg(feature = "test_task_cli")]
    #[structopt(name = "test_task")]
    TestTask(test_task::Section),

    /// Trigger graceful shutdown of Golem
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
                         let endpoint = $ctx.connect_to_app()?;
                         let endpoint = $ctx.unlock_app(endpoint)?;
                         $ctx.block_on(command.run(endpoint))
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
                         let endpoint = $ctx.connect_to_app()?;
                         let eval = command.run($ctx, endpoint);
                         $ctx.block_on(eval)
                    }
                )*
            )?
        }
    }};
}

impl CommandSection {
    pub fn run_command(&self, ctx: &mut CliCtx) -> Result<CommandResponse, crate::context::Error> {
        dispatch_subcommand! {
            on (self, ctx);
            async {
                #[cfg(feature = "concent_cli")]
                CommandSection::Concent,
                CommandSection::Network,
                CommandSection::Envs,
                CommandSection::Incomes,
                CommandSection::Payments,
                CommandSection::Cache,
                CommandSection::Settings,
                CommandSection::Tasks,
                #[cfg(feature = "test_task_cli")]
                CommandSection::TestTask,
                CommandSection::Shutdown,
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
    /// Generates autocomplete script fro given shell
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
    fn run_command(&self) -> Result<CommandResponse, crate::context::Error> {
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
    fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = crate::context::Error> + 'static {
        endpoint
            .as_invoker()
            .rpc_call("golem.graceful_shutdown", &())
            .and_then(|ret: u64| {
                let result = format!("Graceful shutdown triggered result: {}", ret);
                Ok(CommandResponse::Object(result.into()))
            })
            .from_err()
    }
}
