#![allow(dead_code)]

use super::CliArgs;
pub use failure::Error;
use futures::Future;
use serde::Serialize;
use std::convert::TryFrom;
use std::path::PathBuf;

#[derive(Debug)]
pub enum CommandResponse {
    NoOutput,
    Object(serde_json::Value),
    Table {
        headers: Vec<String>,
        values: Vec<serde_json::Value>,
    },
}

impl CommandResponse {
    pub fn object<T: Serialize>(value: T) -> Result<Self, Error> {
        Ok(CommandResponse::Object(serde_json::to_value(value)?))
    }
}

pub struct CliCtx {
    rpc_addr: (String, u16),
    data_dir: PathBuf,
    json_output: bool,
}

impl TryFrom<&CliArgs> for CliCtx {
    type Error = Error;

    fn try_from(value: &CliArgs) -> Result<Self, Self::Error> {
        let data_dir = value.get_data_dir();
        let rpc_addr = value.get_rcp_address()?;
        let json_output = value.json;

        Ok(CliCtx {
            rpc_addr,
            data_dir,
            json_output,
        })
    }
}

impl CliCtx {
    pub fn connect_to_app(
        &mut self,
    ) -> Result<(actix::SystemRunner, impl actix_wamp::RpcEndpoint + Clone), Error> {
        let mut sys = actix::System::new("golemcli");

        let data_dir = self.data_dir.clone();

        let auth_method =
            actix_wamp::challenge_response_auth(move |auth_id| -> Result<_, std::io::Error> {
                let secret_file_path = data_dir.join(format!("crossbar/secrets/{}.tck", auth_id));
                log::debug!("reading secret from: {}", secret_file_path.display());
                Ok(std::fs::read(secret_file_path)?)
            });

        let (address, port) = &self.rpc_addr;

        let endpoint = sys.block_on(
            actix_wamp::SessionBuilder::with_auth("golem", "golemcli", auth_method)
                .create_wss(address, *port),
        )?;

        Ok((sys, endpoint))
    }

    pub fn message(&mut self, message: &str) {
        eprintln!("{}", message);
    }

    pub fn output(&self, resp: CommandResponse) {
        match resp {
            CommandResponse::NoOutput => {}
            CommandResponse::Table { .. } => {}
            CommandResponse::Object(v) => {
                if self.json_output {
                    println!("{}", serde_json::to_string_pretty(&v).unwrap())
                } else {
                    println!("{}", serde_yaml::to_string(&v).unwrap())
                }
            }
        }
    }
}
