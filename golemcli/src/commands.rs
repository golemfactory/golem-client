use crate::context::{CliCtx, CommandResponse};
use std::fmt::{self, Debug};
use structopt::*;

mod account;
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
mod res;
mod settings;
mod subtasks;
mod tasks;
mod terms;
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

    /// Manage environments (unimplemented)
    #[structopt(name = "envs")]
    Envs(envs::Section),

    /// Manage network
    #[structopt(name = "network")]
    Network(network::NetworkSection),

    /// Display incomes (unimplemented)
    #[structopt(name = "incomes")]
    Incomes(incomes::Section),

    /// Display payments (unimplemented)
    #[structopt(name = "payments")]
    Payments(payments::Section),

    /// Display deposit payments (unimplemented)
    #[cfg(feature = "concent_cli")]
    #[structopt(name = "deposit_payments")]
    DepositPayments(deposit_payments::Section),

    /// Manage resources (unimplemented)
    #[structopt(name = "res")]
    Res(res::Section),

    /// Manage settings (unimplemented)
    #[structopt(name = "settings")]
    Settings(settings::Section),

    /// Manage tasks (unimplemented)
    #[structopt(name = "tasks")]
    Tasks(tasks::Section),

    /// Manage subtasks (unimplemented)
    #[structopt(name = "subtasks")]
    Subtasks(subtasks::Section),

    /// Show and accept terms of use (unimplemented)
    #[structopt(name = "terms")]
    Terms(terms::Section),

    /// Manage testing tasks (unimplemented)
    #[cfg(feature = "test_task_cli")]
    #[structopt(name = "test_task")]
    TestTask(test_task::Section),

    #[structopt(name = "_int")]
    #[structopt(raw(setting = "structopt::clap::AppSettings::Hidden"))]
    Internal(InternalSection),
}

impl CommandSection {
    pub fn run_command(&self, ctx: &mut CliCtx) -> Result<CommandResponse, crate::context::Error> {
        match &self {
            CommandSection::Internal(ref command) => command.run_command(),
            CommandSection::Account(ref command) => {
                let (mut sys, endpoint) = ctx.connect_to_app().unwrap();
                sys.block_on(command.run(endpoint))
            }
            CommandSection::Debug(ref command) => {
                let (mut sys, endpoint) = ctx.connect_to_app().unwrap();
                sys.block_on(command.run(endpoint))
            }
            CommandSection::Network(ref command) => {
                let (mut sys, endpoint) = ctx.connect_to_app().unwrap();
                sys.block_on(command.run(endpoint))
            }
            &section => {
                eprintln!("unimplemented command: {:?}", section);
                Ok(CommandResponse::NoOutput)
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
