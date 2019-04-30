use structopt::{clap::arg_enum, StructOpt};

#[derive(StructOpt, Debug)]
pub struct Section {
    filter_by: Option<crate::eth::PaymentStatus>,
    #[structopt(long = "sort")]
    sort_by: Option<Column>,
}

arg_enum! {
    #[derive(Debug)]
    pub enum Column {
        Payer,
        Status,
        Value
    }
}
