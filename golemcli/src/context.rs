#![allow(dead_code)]

use super::CliArgs;
pub use failure::Error;
use futures::Future;
use serde::Serialize;
use std::convert::TryFrom;
use std::path::PathBuf;

pub struct ResponseTable {
    pub columns: Vec<String>,
    pub values: Vec<serde_json::Value>,
}

impl ResponseTable {
    pub fn sort_by(mut self, arg_key: &Option<impl AsRef<str>>) -> Self {
        let key = match arg_key {
            None => return self,
            Some(k) => k.as_ref(),
        };
        let idx =
            match self
                .columns
                .iter()
                .enumerate()
                .find_map(|(idx, v)| if v == key { Some(idx) } else { None })
            {
                None => return self,
                Some(idx) => idx,
            };
        self.values
            .sort_by_key(|v| Some(v.as_array()?.get(idx)?.to_string()));
        self
    }
}

pub trait FormattedObject {
    fn to_json(&self) -> Result<serde_json::Value, Error>;

    fn print(&self) -> Result<(), Error>;
}

pub enum CommandResponse {
    NoOutput,
    Object(serde_json::Value),
    Table {
        columns: Vec<String>,
        values: Vec<serde_json::Value>,
    },
    FormattedObject(Box<dyn FormattedObject>),
}

impl CommandResponse {
    pub fn object<T: Serialize>(value: T) -> Result<Self, Error> {
        Ok(CommandResponse::Object(serde_json::to_value(value)?))
    }
}

impl From<ResponseTable> for CommandResponse {
    fn from(table: ResponseTable) -> Self {
        CommandResponse::Table {
            columns: table.columns,
            values: table.values,
        }
    }
}

pub struct CliCtx {
    rpc_addr: (String, u16),
    data_dir: PathBuf,
    json_output: bool,
    accept_any_prompt: bool,
    net: Option<Net>,
    interactive: bool,
    sys : SystemRunner,
}

impl TryFrom<&CliArgs> for CliCtx {
    type Error = Error;

    fn try_from(value: &CliArgs) -> Result<Self, Self::Error> {
        let data_dir = value.get_data_dir();
        let rpc_addr = value.get_rcp_address()?;
        let json_output = value.json;
        let net = value.net.clone();
        let accept_any_prompt = value.accept_any_prompt;
        let interactive = value.interactive;
        let sys = actix::System::new("golemcli");

        Ok(CliCtx {
            rpc_addr,
            data_dir,
            json_output,
            accept_any_prompt,
            net,
            interactive,
            sys
        })
    }
}

fn wait_for_server(
    endpoint: impl actix_wamp::PubSubEndpoint + Clone + 'static,
) -> impl Future<Item = bool, Error = actix_wamp::Error> {
    use futures::stream::Stream;

    eprintln!("Waiting for server start");
    endpoint
        .subscribe("golem.rpc_ready")
        .into_future()
        .and_then(|(_, _)| Ok(true))
        .map_err(|(e, _)| e)
}

impl CliCtx {

    pub fn block_on<F : Future>(&mut self, f : F) -> Result<F::Item, F::Error> {
        self.sys.block_on(f)
    }

    pub fn unlock_app(&mut self, endpoint : impl actix_wamp::RpcEndpoint + actix_wamp::PubSubEndpoint + Clone + 'static) -> Result<impl actix_wamp::RpcEndpoint + Clone, Error> {
        let is_unlocked = self.block_on(endpoint.as_golem().is_account_unlocked())?;
        let mut wait_for_start = false;

        if !is_unlocked {
            eprintln!("account locked");
            let password = rpassword::read_password_from_tty(Some(
                "Unlock your account to start golem\n\
                 This command will time out in 30 seconds.\n\
                 Password: ",
            ))?;
            let is_valid_password = self.block_on(endpoint.as_golem().set_password(password))?;
            if !is_valid_password {
                return Err(failure::err_msg("invalid password"));
            }
            wait_for_start = true;
        }

        let are_terms_accepted = self.block_on(endpoint.as_golem_terms().are_terms_accepted())?;

        if !are_terms_accepted {
            use crate::terms::*;
            use promptly::Promptable;
            eprintln!("Terms is not accepted");

            loop {
                match TermsQuery::prompt("Accept terms ? [(s)how / (a)ccept / (r)eject]") {
                    TermsQuery::Show => {
                        eprintln!("{}", self.block_on(get_terms_text(&endpoint))?);
                    }
                    TermsQuery::Reject => {
                        return Err(failure::err_msg("terms not accepted"));
                    }
                    TermsQuery::Accept => {
                        break;
                    }
                }
            }
            let enable_monitor = self.prompt_for_acceptance(
                "Enable monitor",
                Some("monitor will be ENABLED"),
                Some("monitor will be DISABLED"),
            );
            let enable_talkback = self.prompt_for_acceptance(
                "Enable talkback",
                Some("talkback will be ENABLED"),
                Some("talkback will be DISABLED"),
            );

            let _ = self.block_on(
                endpoint
                    .as_golem_terms()
                    .accept_terms(Some(enable_monitor), Some(enable_talkback)),
            )?;
            wait_for_start = true;
        }

        if wait_for_start {
            let _ = self.block_on(wait_for_server(endpoint.clone()))?;
        }

        Ok(endpoint)
    }

    pub fn connect_to_app(
        &mut self,
    ) -> Result<impl actix_wamp::RpcEndpoint + actix_wamp::PubSubEndpoint + Clone, Error> {
        let (address, port) = &self.rpc_addr;

        let endpoint = self.block_on(golem_rpc_api::connect_to_app(
            &self.data_dir,
            self.net.clone(),
            Some((address.as_str(), *port)),
        ))?;

        Ok(endpoint)
    }

    pub fn message(&mut self, message: &str) {
        eprintln!("{}", message);
    }

    pub fn output(&self, resp: CommandResponse) {
        match resp {
            CommandResponse::NoOutput => {}
            CommandResponse::Table { columns, values } => {
                if self.json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "headers": columns,
                            "values": values
                        }))
                        .unwrap()
                    )
                } else {
                    print_table(columns, values);
                }
            }
            CommandResponse::Object(v) => {
                if self.json_output {
                    println!("{}", serde_json::to_string_pretty(&v).unwrap())
                } else {
                    match v {
                        serde_json::Value::String(s) => {
                            println!("{}", s);
                        }
                        v => println!("{}", serde_yaml::to_string(&v).unwrap()),
                    }
                }
            }
            CommandResponse::FormattedObject(formatted_object) => {
                if self.json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&formatted_object.to_json().unwrap()).unwrap()
                    )
                } else {
                    formatted_object.print().unwrap()
                }
            }
        }
    }

    pub fn prompt_for_acceptance(
        &self,
        msg: &str,
        msg_on_accept: Option<&str>,
        msg_on_reject: Option<&str>,
    ) -> bool {
        if self.accept_any_prompt && !self.interactive {
            return true;
        }
        let enabled = promptly::prompt_default(msg, true);

        if enabled && msg_on_accept.is_some() {
            eprintln!("\t {}", msg_on_accept.unwrap());
        } else if !enabled && msg_on_reject.is_some() {
            eprintln!("\t {}", msg_on_reject.unwrap());
        }
        enabled
    }

    pub fn get_golem_lock_path(&self, is_mainnet: bool) -> PathBuf {
        let dir = match is_mainnet {
            true => "mainnet",
            false => "rinkeby",
        };

        self.data_dir.join(PathBuf::from(dir).join("LOCK"))
    }
}

pub fn create_table<'a>(columns: impl IntoIterator<Item = &'a str>) -> prettytable::Table {
    use prettytable::*;
    let mut table = Table::new();
    //table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_format(*FORMAT_BASIC);

    table.set_titles(Row::new(
        columns
            .into_iter()
            .map(|c| {
                Cell::new(c)
                    //.with_style(Attr::Bold)
                    .with_style(Attr::ForegroundColor(color::GREEN))
            })
            .collect(),
    ));

    table
}

fn print_table(columns: Vec<String>, values: Vec<serde_json::Value>) {
    use prettytable::*;
    let mut table = Table::new();
    //table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_format(*FORMAT_BASIC);

    table.set_titles(Row::new(
        columns
            .iter()
            .map(|c| {
                Cell::new(c)
                    .with_style(Attr::Bold)
                    .with_style(Attr::ForegroundColor(color::GREEN))
            })
            .collect(),
    ));
    if values.is_empty() {
        let _ = table.add_row(columns.iter().map(|_| Cell::new("")).collect());
    }
    for row in values {
        if let Some(row_items) = row.as_array() {
            use serde_json::Value;

            let row_strings = row_items
                .iter()
                .map(|v| match v {
                    Value::String(s) => s.to_string(),
                    Value::Null => "".into(),
                    v => v.to_string(),
                })
                .collect();
            table.add_row(row_strings);
        }
    }
    let _ = table.printstd();
}

use golem_rpc_api::core::AsGolemCore;
use golem_rpc_api::terms::AsGolemTerms;
use golem_rpc_api::Net;
use prettytable::{format, format::TableFormat, Table};
use std::thread::sleep;
use std::time::Duration;
use actix::SystemRunner;
use actix_wamp::PubSubEndpoint;
lazy_static::lazy_static! {

    pub static ref FORMAT_BASIC: TableFormat = format::FormatBuilder::new()
        .column_separator('│')
        .borders('│')
        .separators(
            &[format::LinePosition::Top],
            format::LineSeparator::new('─', '┬', '┌', '┐')
        )
        .separators(
            &[format::LinePosition::Title],
            format::LineSeparator::new('─', '┼', '├', '┤')
        )
        .separators(
            &[format::LinePosition::Bottom],
            format::LineSeparator::new('─', '┴', '└', '┘')
        )
        .padding(2, 2)
        .build();
}

pub fn format_key(s: &str, full: bool) -> String {
    if full {
        return s.to_string();
    }

    let key_size = s.len();
    if key_size < 32 {
        s.into()
    } else {
        format!("{}...{}", &s[..16], &s[(key_size - 16)..])
    }
}
