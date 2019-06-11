use crate::context::{CliCtx, CommandResponse};
use std::fmt::{self, Debug};
use structopt::*;

mod account;
mod cache;
#[cfg(feature = "concent_cli")]
mod concent;
#[cfg(feature = "debug_cli")]
mod debug;
#[cfg(feature = "concent_cli")]
mod deposit_payments;
mod envs;
mod incomes;
mod network;
mod payments;
mod settings;
mod tasks;
mod terms;
mod status;
#[cfg(feature = "test_task_cli")]
mod test_task;

#[derive(StructOpt, Debug)]
pub enum CommandSection {
    /// Manage account
    #[structopt(name = "account")]
    Account(account::AccountSection),

    /// Concent Service (unimplemented)
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

    /// Display incomes
    #[structopt(name = "incomes")]
    Incomes(incomes::Section),

    /// Display payments
    #[structopt(name = "payments")]
    Payments(payments::Section),

    /// Display deposit payments
    #[cfg(feature = "concent_cli")]
    #[structopt(name = "deposit_payments")]
    DepositPayments(deposit_payments::Section),

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
            $async_command:path,)*
        })?
        $(async_with_cxt {
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
                         let (mut sys, endpoint) = $ctx.connect_to_app()?;
                         sys.block_on(command.run(endpoint))
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
                         let (mut sys, endpoint) = $ctx.connect_to_app()?;
                         sys.block_on(command.run($ctx, endpoint))
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
                #[cfg(feature = "debug_cli")]
                CommandSection::Debug,
                CommandSection::Network,
                CommandSection::Envs,
                CommandSection::Incomes,
                CommandSection::Payments,
                #[cfg(feature = "concent_cli")]
                CommandSection::DepositPayments,
                CommandSection::Cache,
                CommandSection::Settings,
                CommandSection::Tasks,
                CommandSection::Status,
                #[cfg(feature = "test_task_cli")]
                CommandSection::TestTask,

            }
            async_with_cxt {
                CommandSection::Account,
                CommandSection::Terms,
            }
            sync {
                CommandSection::Internal
            }
        }

        /*match self {
            CommandSection::Internal(ref command) => command.run_command(),
            CommandSection::Account(ref command) => {
                let (mut sys, endpoint) = ctx.connect_to_app()?;
                sys.block_on(command.run(endpoint))
            }

            CommandSection::Concent(command) => {
                let (mut sys, endpoint) = ctx.connect_to_app()?;
                sys.block_on(command.run(endpoint))
            }

            #[cfg(feature = "debug_cli")]
            CommandSection::Debug(ref command) => {
                let (mut sys, endpoint) = ctx.connect_to_app()?;
                sys.block_on(command.run(endpoint))
            }

            CommandSection::Network(ref command) => {
                let (mut sys, endpoint) = ctx.connect_to_app()?;
                sys.block_on(command.run(endpoint))
            }
            CommandSection::Terms(ref command) => {
                let (mut sys, endpoint) = ctx.connect_to_app()?;
                sys.block_on(command.run(endpoint))
            }

            CommandSection::TestTask(ref command) => {
                let (mut sys, endpoint) = ctx.connect_to_app()?;
                sys.block_on(command.run(endpoint))
            }
            _ => unimplemented!()
        }*/
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
