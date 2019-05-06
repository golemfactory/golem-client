use crate::context::*;
use futures::prelude::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Clear provider / requestor resources
    #[structopt(name = "clear")]
    Clear,
    /// Show information on used resources
    #[structopt(name = "show")]
    Show,
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
