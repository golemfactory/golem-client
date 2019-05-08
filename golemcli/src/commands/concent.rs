use crate::context::*;
use futures::{future, Future};
use std::str::FromStr;
use structopt::{clap::arg_enum, StructOpt};

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Soft Switch
    #[structopt(name = "switch")]
    Switch(Switch),

    /// Terms of Use
    #[structopt(name = "terms")]
    Terms(Terms),
}

#[derive(StructOpt, Debug)]
pub enum Switch {
    #[structopt(name = "turn")]
    Turn {
        #[structopt(
            parse(try_from_str),
            raw(possible_values = "&[\"on\",\"off\"]", case_insensitive = "true")
        )]
        on_off: OnOff,
    },
    #[structopt(name = "is_on")]
    IsOn,
}

#[derive(StructOpt, Debug)]
pub enum Terms {
    #[structopt(name = "accept")]
    Accept,
    #[structopt(name = "show")]
    Show,
}

arg_enum! {
    #[derive(Debug)]
    pub enum OnOff {
        On,
        Off
    }
}

impl Section {
    pub fn run(
        &self,
        _endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        futures::future::err(unimplemented!())
    }
}
