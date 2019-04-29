
use structopt::clap::arg_enum;
use serde::Serialize;

arg_enum! {
    #[derive(Debug, Serialize)]
    pub enum Currency {
        ETH,
        GNT
    }
}