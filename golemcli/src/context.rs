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
}

impl TryFrom<&CliArgs> for CliCtx {
    type Error = Error;

    fn try_from(value: &CliArgs) -> Result<Self, Self::Error> {
        let data_dir = value.get_data_dir();
        let rpc_addr = value.get_rcp_address()?;
        let json_output = value.json;
        let net = value.net.clone();
        let accept_any_prompt = value.accept_any_prompt;

        Ok(CliCtx {
            rpc_addr,
            data_dir,
            json_output,
            accept_any_prompt,
            net,
        })
    }
}

impl CliCtx {
    pub fn connect_to_app(
        &mut self,
    ) -> Result<(actix::SystemRunner, impl actix_wamp::RpcEndpoint + Clone), Error> {
        let mut sys = actix::System::new("golemcli");
        let (address, port) = &self.rpc_addr;

        let endpoint = sys.block_on(golem_rpc_api::connect_to_app(
            &self.data_dir,
            self.net.clone(),
            Some((address.as_str(), *port)),
        ))?;

        Ok((sys, endpoint))
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

    pub fn prompt_for_acceptance(&self, msg: &str, msg_on_accept: Option<&str>,
                                 msg_on_reject: Option<&str>) -> bool {
        if self.accept_any_prompt {
            return true;
        }
        let enabled = promptly::prompt_default( msg, true);

        if enabled && msg_on_accept.is_some() {
            eprintln!("\t {}", msg_on_accept.unwrap());
        } else if !enabled && msg_on_reject.is_some() {
            eprintln!("\t {}", msg_on_reject.unwrap());
        }
        enabled
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

use golem_rpc_api::Net;
use prettytable::{format, format::TableFormat, Table};
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