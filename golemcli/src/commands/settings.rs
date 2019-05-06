use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::core::AsGolemCore;
use structopt::{clap, StructOpt};

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Change settings (unimplemented)
    #[structopt(name = "set")]
    Set {
        /// Setting name
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
                .and_then(|settings| CommandResponse::object(settings)),
        )
    }

    // TODO: Convert types
    pub fn set(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        key: &str,
        value: &str,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        Box::new(
            endpoint
                .as_golem()
                .update_setting(key.into(), serde_json::json!(value))
                .from_err()
                .and_then(|()| CommandResponse::object("Updated")),
        )
    }
}
