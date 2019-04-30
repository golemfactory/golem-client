#![allow(unused_imports)]

use crate::context::CliCtx;
use actix::prelude::*;
use actix_wamp::{Error, RpcCallRequest, RpcEndpoint};
use std::convert::TryInto;
use std::fmt::Debug;
use std::path::PathBuf;
use structopt::*;

pub(crate) mod commands;
pub(crate) mod context;
pub(crate) mod eth;
pub(crate) mod rpc;

#[derive(StructOpt, Debug)]
#[structopt(raw(global_setting = "structopt::clap::AppSettings::ColoredHelp"))]
#[structopt(raw(global_setting = "structopt::clap::AppSettings::VersionlessSubcommands"))]
struct CliArgs {
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

    #[structopt(subcommand)]
    command: Option<commands::CommandSection>,
}

impl CliArgs {
    pub fn get_data_dir(&self) -> PathBuf {
        match &self.data_dir {
            Some(data_dir) => data_dir.join("rinkeby"),
            None => appdirs::user_data_dir(Some("golem"), None, false)
                .unwrap()
                .join("default")
                .join("rinkeby"),
        }
    }

    pub fn get_rcp_address(&self) -> failure::Fallible<(String, u16)> {
        let address = match &self.address {
            Some(a) => a.as_str(),
            None => "127.0.0.1",
        };

        Ok((address.into(), self.port.unwrap_or(61000)))
    }

    fn run_command(&self) {
        let mut ctx: CliCtx = self.try_into().unwrap();
        match &self.command {
            None => {
                <Self as StructOpt>::clap().print_help().unwrap();
                eprintln!();
            }
            Some(command) => {
                let resp = command.run_command(&mut ctx);
                ctx.output(resp.unwrap());
            }
        }
    }
}

fn main() -> failure::Fallible<()> {
    let args = CliArgs::from_args();

    flexi_logger::Logger::with_env_or_str("error")
        .start()
        .unwrap();

    args.run_command();
    Ok(())
}
