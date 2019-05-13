use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::comp::AsGolemComp;
use golem_rpc_api::terms::*;
use std::path::{Path, PathBuf};
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

    #[structopt(name = "status")]
    Status,
}

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            Section::Run { task_file } => Box::new(self.do_run(endpoint, task_file)),
            Section::Abort => Box::new(self.abort(endpoint)),
            Section::Status => Box::new(self.status(endpoint)),
        }
    }

    fn do_run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_file: &Path,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        use std::fs;

        fs::OpenOptions::new()
            .read(true)
            .open(task_file)
            .into_future()
            .and_then(|file| Ok(serde_json::from_reader(file)?))
            .from_err()
            .and_then(move |task_spec| endpoint.as_golem_comp().run_test_task(task_spec).from_err())
            .and_then(|result| CommandResponse::object(if result { "Success" } else { "Error" }))
    }

    fn abort(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .abort_test_task()
            .from_err()
            .and_then(|result| {
                CommandResponse::object(if result {
                    "Success"
                } else {
                    "There was no test task to abort"
                })
            })
    }

    fn status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .abort_test_task()
            .from_err()
            .and_then(|status| CommandResponse::object(status))
    }
}
