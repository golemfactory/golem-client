use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::settings::DynamicSetting;
use golem_rpc_api::{core::AsGolemCore, settings, Map};
use serde_json::Value;
use std::collections::btree_map::BTreeMap;
use std::collections::{HashMap, HashSet};
use structopt::{clap, StructOpt};

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Change settings (unimplemented)
    #[structopt(name = "set")]
    Set {
        /// Setting name
        #[structopt(raw(possible_values = "settings::NAMES",))]
        key: String,
        /// Setting value
        value: String,
    },
    /// Show current settings
    #[structopt(name = "show")]
    //#[structopt(raw(group = "show_opt_group()"))]
    Show {
        /// Show basic settings
        #[structopt(long)]
        basic: bool,

        /// Show provider settings
        #[structopt(long)]
        provider: bool,

        /// Show requestor settings
        #[structopt(long)]
        requestor: bool,
    },
}

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        match self {
            &Section::Show {
                basic,
                provider,
                requestor,
            } => self.show(endpoint, basic, provider, requestor),
            Section::Set { key, value } => self.set(endpoint, key, value),
        }
    }

    pub fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        basic: bool,
        provider: bool,
        requestor: bool,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        Box::new(endpoint.as_golem().get_settings().from_err().and_then(
            move |settings: Map<String, Value>| {
                Ok(CommandResponse::FormattedObject({
                    if basic || provider || requestor {
                        let mut filtered_settings: Map<String, serde_json::Value> = Map::new();

                        let mut add_settings = |list: Vec<
                            &'static (dyn DynamicSetting + 'static),
                        >|
                         -> Result<(), Error> {
                            for setting in list {
                                if let Some(value) = settings.get(setting.name()) {
                                    filtered_settings.insert(setting.name().into(), value.clone());
                                }
                            }
                            Ok(())
                        };

                        if basic {
                            add_settings(settings::general::list().collect())?;
                        }
                        if provider {
                            add_settings(settings::provider::list().collect())?;
                        }
                        if requestor {
                            add_settings(settings::requestor::list().collect())?;
                        }

                        Box::new(FormattedSettings(filtered_settings))
                    } else {
                        Box::new(FormattedSettings(settings))
                    }
                }))
            },
        ))
    }

    pub fn set(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        key: &str,
        value: &str,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        let key = settings::from_name(key).unwrap();

        Box::new(
            endpoint
                .as_golem()
                .update_setting_dyn(key, value)
                .from_err()
                .and_then(|()| CommandResponse::object("Updated")),
        )
    }
}

struct FormattedSettings(Map<String, Value>);

impl FormattedSettings {
    fn dump_section(
        &self,
        table: &mut prettytable::Table,
        keys: &mut HashSet<&'static str>,
        section_name: &str,
        settings: impl Iterator<Item = &'static dyn DynamicSetting>,
    ) -> Result<(), Error> {
        use prettytable::*;

        let mut header = false;
        for setting in settings {
            if let Some(v) = self.0.get(setting.name()) {
                if !header {
                    table.add_empty_row();
                    table.add_row(Row::new(vec![Cell::new(section_name)
//                        .with_style(Attr::Dim)
                        .with_style(Attr::Underline(true))
                        .with_style(Attr::ForegroundColor(color::YELLOW))]));
                    table.add_empty_row();
                    header = true;
                }
                table.add_row(row![
                    format!("{} [{}]", setting.description(), setting.name()),
                    setting.display_value(v)?,
                    setting.validation_desc()
                ]);
                keys.insert(setting.name());
            }
        }

        Ok(())
    }
}

impl FormattedObject for FormattedSettings {
    fn to_json(&self) -> Result<Value, Error> {
        Ok(serde_json::to_value(&self.0)?)
    }

    fn print(&self) -> Result<(), Error> {
        use prettytable::*;

        let mut table = create_table(vec!["description [name]", "value", "type"]);
        let mut keys = HashSet::new();

        self.dump_section(&mut table, &mut keys, "General", settings::general::list())?;
        self.dump_section(
            &mut table,
            &mut keys,
            "Requestor",
            settings::requestor::list(),
        )?;
        self.dump_section(
            &mut table,
            &mut keys,
            "Provider",
            settings::provider::list(),
        )?;

        table.add_empty_row();
        table.add_row(Row::new(vec![Cell::new("Other")
            .with_style(Attr::Underline(true))
            .with_style(Attr::ForegroundColor(color::YELLOW))]));
        table.add_empty_row();
        table.add_empty_row();
        for (name, value) in &self.0 {
            if !keys.contains(name.as_str()) {
                if let Some(setting) = settings::from_name(name) {
                    table.add_row(row![
                        format!("{} [{}]", setting.description(), setting.name()),
                        setting.display_value(value)?,
                        setting.validation_desc()
                    ]);
                } else {
                    table.add_row(row![name, value, ""]);
                }
            }
        }
        table.printstd();
        Ok(())
    }
}
