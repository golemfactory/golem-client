use crate::context::*;
use futures::prelude::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Section {
    filter_by: Option<crate::eth::PaymentStatus>,
    //#[structopt(long="sort")]
    //sort_by : Option<Column>
}

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        match self {
            _ => futures::future::err(unimplemented!()),
        }
    }
}
