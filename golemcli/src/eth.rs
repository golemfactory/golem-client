use serde::Serialize;
use structopt::clap::arg_enum;

arg_enum! {
    #[derive(Debug, Serialize)]
    pub enum Currency {
        ETH,
        GNT
    }
}

arg_enum! {
    #[derive(Debug)]
    pub enum PaymentStatus {
        awaiting,
        confirmed
    }
}