use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::settings::DynamicSetting;
use golem_rpc_api::{core::AsGolemCore, settings, Map};
use std::collections::btree_map::BTreeMap;
use std::collections::HashMap;
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

/*fn show_opt_group() -> clap::ArgGroup<'static> {
    clap::ArgGroup::with_name("filter")
        .args(&["basic", "provider", "requestor"])
        .multiple(false)
        .required(false)

}*/

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        eprintln!("me={:?}", self);
        match self {
            &Section::Show {
                basic,
                provider,
                requestor,
            } => self.show(endpoint, basic, provider, requestor),
            Section::Set { key, value } => self.set(endpoint, key, value),
        }
    }

    // TODO: Implement filtering for basic, provider, requestor flag.
    pub fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        basic: bool,
        provider: bool,
        requestor: bool,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        Box::new(
            endpoint
                .as_golem()
                .get_settings()
                .from_err()
                .and_then(move |settings| {
                    CommandResponse::object({
                        if basic || provider || requestor {
                            let mut filtered_settings: Map<String, serde_json::Value> = Map::new();
                            if basic {
                                for setting in settings::general::list() {
                                    if let Some(value) = settings.get(setting.name()) {
                                        filtered_settings.insert(
                                            setting.name().into(),
                                            serde_json::json!(setting.display_value(&value)?),
                                        );
                                    }
                                }
                            }
                            filtered_settings
                        } else {
                            settings
                        }
                    })
                }),
        )
    }

    // TODO: Convert types
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
