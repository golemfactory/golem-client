use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct  Section {
    filter_by : Option<crate::eth::PaymentStatus>,
    //#[structopt(long="sort")]
    //sort_by : Option<Column>
}