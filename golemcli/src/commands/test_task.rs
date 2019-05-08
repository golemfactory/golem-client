use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::terms::*;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    // Abort testing task
    #[structopt(name = "abort")]
    Abort,

    /// Run testing task. It accepts a file like 'tasks create'.
    #[structopt(name = "run")]
    Run {
        /// Task file
        task_file: PathBuf,
    },
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
