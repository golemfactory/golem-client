#![allow(unused_imports)]

use crate::context::CliCtx;
use actix::prelude::*;
use actix_wamp::{Error, RpcCallRequest, RpcEndpoint};
use failure::Fallible;
use fs2::FileExt;
use golem_rpc_api::Net;
use std::convert::TryInto;
use std::fmt::Debug;
use std::fs::File;
use std::path::PathBuf;
use std::{thread, time};
use structopt::*;

pub(crate) mod account;
pub(crate) mod commands;
pub(crate) mod context;
pub(crate) mod eth;
pub(crate) mod formaters;
pub(crate) mod terms;
pub(crate) mod utils;

#[macro_use]
pub(crate) mod component_response;

#[cfg(feature = "interactive_cli")]
mod interactive;
mod version;

#[derive(StructOpt, Debug)]
#[structopt(raw(global_setting = "structopt::clap::AppSettings::ColoredHelp"))]
#[structopt(raw(global_setting = "structopt::clap::AppSettings::VersionlessSubcommands"))]
#[structopt(raw(global_setting = "structopt::clap::AppSettings::DisableVersion"))]
#[structopt(raw(setting = "clap::AppSettings::DeriveDisplayOrder"))]
struct CliArgs {
    #[cfg(feature = "interactive_cli")]
    /// Enter interactive mode
    #[structopt(short, long)]
    interactive: bool,

    /// Golem node's RPC address
    #[structopt(short, long, name = "rpc_address")]
    #[structopt(raw(display_order = "500"))]
    #[structopt(raw(set = "structopt::clap::ArgSettings::Global"))]
    address: Option<String>,

    /// Golem node's RPC port
    #[structopt(short, long, name = "rpc_port")]
    #[structopt(raw(display_order = "500"))]
    #[structopt(raw(set = "structopt::clap::ArgSettings::Global"))]
    port: Option<u16>,

    /// Golem node's data dir
    #[structopt(short, long = "datadir")]
    #[structopt(raw(set = "structopt::clap::ArgSettings::Global"))]
    data_dir: Option<PathBuf>,

    /// Return results in JSON format
    #[structopt(long)]
    #[structopt(raw(display_order = "500"))]
    #[structopt(raw(set = "structopt::clap::ArgSettings::Global"))]
    json: bool,

    #[structopt(long, short)]
    net: Option<Net>,

    /// Answer `YES` to any prompt without an user action
    #[structopt(short = "y", long = "assume-yes")]
    #[structopt(raw(display_order = "1000"))]
    #[structopt(raw(set = "structopt::clap::ArgSettings::Global"))]
    accept_any_prompt: bool,

    #[structopt(subcommand)]
    command: Option<commands::CommandSection>,

    /// Prints version information
    #[structopt(long, short = "V")]
    version: bool,
}

impl CliArgs {
    pub fn get_data_dir(&self) -> PathBuf {
        match &self.data_dir {
            Some(data_dir) => data_dir.to_owned(),
            None => appdirs::user_data_dir(Some("golem"), Some("golem"), false)
                .unwrap()
                .join("default"),
        }
    }

    pub fn get_rcp_address(&self) -> Fallible<(String, u16)> {
        let address = match &self.address {
            Some(a) => a.as_str(),
            None => "127.0.0.1",
        };

        Ok((address.into(), self.port.unwrap_or(61000)))
    }

    #[cfg(not(feature = "interactive_cli"))]
    fn no_command(&self) {
        <Self as StructOpt>::clap().print_help().unwrap();
        eprintln!();
    }

    #[cfg(not(feature = "interactive_cli"))]
    fn post_command(&self, _: &mut CliCtx) {}

    #[cfg(feature = "interactive_cli")]
    fn no_command(&self) {
        if !self.interactive {
            <Self as StructOpt>::clap().print_help().unwrap();
            eprintln!();
        }
    }

    #[cfg(feature = "interactive_cli")]
    async fn post_command(&self, ctx: &mut CliCtx) {
        if self.interactive {
            interactive::interactive_shell(ctx).await
        }
    }

    async fn run_command(&self) -> Fallible<()> {
        let mut ctx: CliCtx = self.try_into()?;
        match &self.command {
            None => self.no_command(),
            Some(command) => {
                let resp = command.run_command(&mut ctx).await?;
                ctx.output(resp);
            }
        }
        Ok(self.post_command(&mut ctx).await)
    }
}

#[actix_rt::main]
async fn main() -> Fallible<()> {
    let args = CliArgs::from_args();

    if args.version {
        version::print();
        return Ok(());
    }

    flexi_logger::Logger::with_env_or_str("error")
        .start()
        .unwrap();

    args.run_command().await
}
