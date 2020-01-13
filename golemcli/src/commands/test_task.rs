use crate::context::*;
use failure::Fallible;
use futures::prelude::*;
use golem_rpc_api::comp::AsGolemComp;
use golem_rpc_api::terms::*;
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Run testing task. It accepts a file like 'tasks create'.
    #[structopt(name = "run")]
    Run {
        /// Task file
        task_file: PathBuf,
    },
    /// Dump a task template
    #[structopt(name = "template")]
    Template {
        #[structopt(raw(possible_values = "super::tasks::TASK_TYPES",))]
        task_type: String,
    },

    /// Show test_task status
    #[structopt(name = "status")]
    Status,

    /// Abort a task. It will delete a task details
    #[structopt(name = "abort")]
    Abort,
}

impl Section {
    pub async fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        match self {
            Section::Run { task_file } => self.do_run(endpoint, task_file).await,
            Section::Abort => self.abort(endpoint).await,
            Section::Status => self.status(endpoint).await,
            Section::Template { task_type } => super::tasks::template(task_type).await,
        }
    }

    async fn do_run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_file: &Path,
    ) -> Fallible<CommandResponse> {
        let task_spec = serde_json::from_reader(fs::OpenOptions::new().read(true).open(task_file)?);
        let result = endpoint.as_golem_comp().run_test_task(task_spec).await?;

        CommandResponse::object(if result { "Success" } else { "Error" })
    }

    async fn abort(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        CommandResponse::object(if endpoint.as_golem_comp().abort_test_task().await? {
            "Success"
        } else {
            "There was no test task to abort"
        })
    }

    async fn status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        CommandResponse::object(endpoint.as_golem_comp().abort_test_task().await?)
    }
}
