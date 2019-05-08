use crate::context::*;
use futures::prelude::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Restart a subtask
    Restart,
    /// Show subtask details
    Show,
}

impl Section {
    pub fn run(
        &self,
        _endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        match self {
            _ => futures::future::err(unimplemented!()),
        }
    }
}
