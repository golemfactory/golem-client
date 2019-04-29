use actix::prelude::*;
use actix_wamp::{Error, RpcCallRequest, RpcEndpoint};
use std::path::PathBuf;
use std::process::Command;
use structopt::*;

mod data_dir;

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
    command: Option<CommandSection>,
}

#[derive(StructOpt, Debug)]
enum CommandSection {
    /// Manage network
    #[structopt(name = "network")]
    Network(NetworkSection),

    /// Manage account
    #[structopt(name = "account")]
    Account(AccountSection),
}

#[derive(StructOpt, Debug)]
enum NetworkSection {
    /// Block provider
    #[structopt(name = "block")]
    Block {
        /// ID of a node
        node_id: String,
    },
    /// Connect to a node
    #[structopt(name = "connect")]
    Connect {
        /// Remote IP address
        ip: String,
        /// Remote TCP port
        port: u16,
    },
    /// Show known nodes
    #[structopt(name = "dht")]
    Dht,
    /// Show connected nodes
    #[structopt(name = "show")]
    Show {
        /// Show full table contents
        #[structopt(long)]
        full: bool,

        /// Sort nodes
        /// ip, port, id, name
        #[structopt(long)]
        sort: Option<String>,
    },
    /// Show client status
    #[structopt(name = "status")]
    Status,
}

#[derive(StructOpt, Debug)]
enum AccountSection {
    /// Display account & financial info
    #[structopt(name = "info")]
    Info,
    /// Trigger graceful shutdown of your golem
    #[structopt(name = "shutdown")]
    Shutdown,
    /// Unlock account, will prompt for your password
    #[structopt(name = "unlock")]
    Unlock,
}

impl CliArgs {
    fn get_data_dir(&self) -> PathBuf {
        match &self.data_dir {
            Some(data_dir) => data_dir.join("rinkeby"),
            None => appdirs::user_data_dir(Some("golem"), None, false)
                .unwrap()
                .join("rinkeby"),
        }
    }

    fn connect_to_app(
        &mut self,
        sys: &mut SystemRunner,
    ) -> Result<impl RpcEndpoint + Clone, Error> {
        let data_dir = self.get_data_dir();

        let auth_method =
            actix_wamp::challenge_response_auth(move |auth_id| -> Result<_, std::io::Error> {
                Ok(std::fs::read(
                    data_dir.join(format!("crossbar/secrets/{}.tck", auth_id)),
                )?)
            });

        let address = match &self.address {
            Some(a) => a.as_str(),
            None => "127.0.0.1",
        };

        sys.block_on(
            actix_wamp::SessionBuilder::with_auth("golem", "golemcli", auth_method)
                .create_wss(address, self.port.unwrap_or(61000)),
        )
    }

    fn run_command(&self, sys: &mut SystemRunner, endpoint : impl RpcEndpoint + Clone) {
        match self.command {
            None => <Self as StructOpt>::clap().print_help().unwrap(),
            _ => Self::clap().print_help().unwrap(),
        }
        eprintln!();
    }
}

fn main() -> failure::Fallible<()> {
    let mut args = CliArgs::from_args();

    eprintln!("args={:?}", args);

    flexi_logger::Logger::with_env_or_str("info")
        .start()
        .unwrap();

    let mut sys = System::new("golemcli");
    use actix_wamp::RpcEndpoint;
    let endpoint = args.connect_to_app(&mut sys)?;

    args.run_command(&mut sys, endpoint.clone());
    //let _ = sys.run();

    Ok(())
}
