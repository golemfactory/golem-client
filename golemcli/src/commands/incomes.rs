use structopt::{StructOpt, clap::arg_enum};

#[derive(StructOpt, Debug)]
pub struct Section {
    filter_by : Option<crate::eth::PaymentStatus>,
    #[structopt(long="sort")]
    sort_by : Option<Column>
}

arg_enum! {
    #[derive(Debug)]
    pub enum Column {
        payer,
        status,
        value
    }
}
