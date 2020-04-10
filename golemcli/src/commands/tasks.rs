use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::prelude::*;
use humantime::format_duration;
use openssl::rsa::Padding;
use structopt::StructOpt;

use golem_rpc_api::comp::{AsGolemComp, StatsCounters, SubtaskStats, TaskInfo};
use serde_json::json;

use crate::context::*;
use failure::Fallible;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Lists current tasks + task_id show task details
    #[structopt(name = "show")]
    Show {
        /// Task identifier
        task_id: Option<String>,
        /// Show only current tasks
        #[structopt(long)]
        current: bool,

        /// Sort tasks
        #[structopt(long)]
        sort: Option<String>,
    },
    /// Lists current tasks
    #[structopt(name = "list")]
    List {
        /// Show only current tasks
        #[structopt(long)]
        current: bool,
        /// Sort tasks
        #[structopt(long)]
        sort: Option<String>,
    },

    /// Dump a task template
    #[structopt(name = "template")]
    Template {
        #[structopt(raw(possible_values = "TASK_TYPES",))]
        task_type: String,
    },

    /// Create a task from file. Note: Some client-side validation is performed, but might be incomplete.
    /// This will change in the future
    #[structopt(name = "create")]
    Create {
        /// Task file
        file_name: PathBuf,
        #[structopt(long = "dry-run", short = "n")]
        dry_run: bool,
        ///  Output file
        #[structopt(short, long = "out-file")]
        out_file: Option<PathBuf>,
    },
    /// Restart a task
    #[structopt(name = "restart")]
    Restart {
        /// Task identifier
        task_id: String,
    },

    /// Abort a task
    #[structopt(name = "abort")]
    Abort {
        /// Task identifier
        task_id: String,
    },
    /// Delete a task
    #[structopt(name = "delete")]
    Delete {
        /// Task identifier
        task_id: String,
    },
    /// Dump an existing task
    #[structopt(name = "dump")]
    Dump {
        /// Task identifier
        task_id: String,
        ///  Output file
        out_file: Option<PathBuf>,
    },
    /// Deletes all tasks
    #[structopt(name = "purge")]
    Purge,
    /// Show statistics for tasks
    #[structopt(name = "stats")]
    Stats,
    /// Show sub-tasks
    #[structopt(name = "subtasks")]
    Subtasks(SubtaskCommand),

    /// Show statistics of all unsupported subtasks
    #[structopt(name = "unsupport")]
    Unsupport { last_days: Option<i32> },
}

#[derive(StructOpt, Debug)]
pub enum SubtaskCommand {
    /// Lists subtasks in given task
    #[structopt(name = "list")]
    List { task_id: String },
    /// Show sub-tasks
    #[structopt(name = "show")]
    Show { subtask_id: String },
    /// Restart given subtasks from a task
    #[structopt(name = "restart")]
    Restart {
        task_id: String,
        subtask_ids: Vec<String>,
    },
}

pub const TASK_TYPES: &[&str] = &["blender", "wasm", "glambda"];

impl Section {
    pub async fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> failure::Fallible<CommandResponse> {
        match self {
            Section::Abort { task_id } => self.abort(endpoint, task_id).await,
            Section::Create {
                file_name,
                dry_run,
                out_file,
            } => {
                if *dry_run {
                    self.create_dry_run(endpoint, file_name, out_file).await
                } else {
                    self.create(endpoint, &file_name, out_file).await
                }
            }
            Section::Delete { task_id } => self.delete(endpoint, task_id).await,
            Section::Dump { task_id, out_file } => self.dump(endpoint, task_id, out_file).await,
            Section::Purge => self.purge(endpoint).await,
            Section::Restart { task_id } => self.restart(endpoint, task_id).await,
            Section::List { current, sort } => self.show(endpoint, &None, *current, sort).await,
            Section::Show {
                task_id,
                current,
                sort,
            } => self.show(endpoint, task_id, *current, sort).await,
            Section::Template { task_type } => template(task_type).await,
            Section::Stats => self.stats(endpoint).await,
            Section::Subtasks(subtask_command) => subtask_command.run(endpoint).await,

            Section::Unsupport { last_days } => self.unsupport(endpoint, last_days).await,
        }
    }

    async fn create(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        file_name: &Path,
        out_file: &Option<PathBuf>,
    ) -> Fallible<CommandResponse> {
        use std::fs;

        let out_file = out_file.clone();

        let task_spec =
            serde_json::from_reader(fs::OpenOptions::new().read(true).open(file_name)?)?;
        let task_id = endpoint.as_golem_comp().create_task(task_spec).await?;

        if let Some(out_file) = out_file {
            fs::write(out_file, task_id)?;
            Ok(CommandResponse::NoOutput)
        } else {
            CommandResponse::object(task_id)
        }
    }

    async fn create_dry_run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        file_name: &Path,
        out_file: &Option<PathBuf>,
    ) -> Fallible<CommandResponse> {
        use std::fs;
        let out_file = out_file.clone();

        let task_spec =
            serde_json::from_reader(fs::OpenOptions::new().read(true).open(file_name)?)?;

        let v = endpoint.as_golem_comp().create_dry_run(task_spec).await?;

        if let Some(out_file) = out_file {
            serde_json::to_writer_pretty(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(out_file)?,
                &v,
            )?;
            Ok(CommandResponse::NoOutput)
        } else {
            CommandResponse::object(v)
        }
    }

    async fn abort(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_id: &str,
    ) -> Fallible<CommandResponse> {
        endpoint.as_golem_comp().abort_task(task_id.into()).await?;
        CommandResponse::object("Completed")
    }

    async fn delete(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_id: &str,
    ) -> Fallible<CommandResponse> {
        endpoint.as_golem_comp().delete_task(task_id.into()).await?;
        CommandResponse::object("Completed")
    }

    async fn purge(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        endpoint.as_golem_comp().purge_tasks().await?;
        CommandResponse::object("Completed")
    }

    async fn restart(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_id: &str,
    ) -> Fallible<CommandResponse> {
        let (task_id, err_msg) = endpoint
            .as_golem_comp()
            .restart_task(task_id.into())
            .await?;
        CommandResponse::object(task_id.or(err_msg))
    }

    async fn dump(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_id: &str,
        out_file: &Option<PathBuf>,
    ) -> Fallible<CommandResponse> {
        let v = endpoint.as_golem_comp().get_task(task_id.into()).await?;

        if let Some(out_file) = out_file {
            serde_json::to_writer_pretty(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(out_file)?,
                &v,
            )?;
        } else {
            println!("{}", serde_json::to_string_pretty(&v)?)
        }
        Ok(CommandResponse::NoOutput)
    }

    async fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + 'static,
        opt_task_id: &Option<String>,
        _current: bool,
        sort: &Option<String>,
    ) -> Fallible<CommandResponse> {
        let sort = sort.clone();

        if let Some(task_id) = opt_task_id {
            let task = endpoint.as_golem_comp().get_task(task_id.clone()).await?;
            CommandResponse::object(task)
        } else {
            let tasks = endpoint.as_golem_comp().get_tasks().await?;

            let columns = vec![
                "id".into(),
                "ETA".into(),
                "subtasks_count".into(),
                "status".into(),
                "completion".into(),
            ];
            let values = tasks
                .into_iter()
                .map(|task| {
                    serde_json::json!([
                        task.id,
                        task.time_remaining.map(seconds_to_human),
                        task.subtasks_count,
                        task.status,
                        task.progress.map(fraction_to_percent),
                    ])
                })
                .collect();
            Ok(ResponseTable { columns, values }.sort_by(&sort).into())
        }
    }

    async fn stats(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        let stats = endpoint.as_golem_comp().get_tasks_stats().await?;

        let columns: Vec<String> = vec![
            "".into(),
            "global".into(),
            "session".into(),
            "config".into(),
        ];
        let values = vec![
            serde_json::json!(["provider state", "", "", stats.provider_state,]),
            serde_json::json!(["in network", stats.in_network, "", "",]),
            serde_json::json!(["supported", stats.supported, "", "",]),
            serde_json::json!([
                "accepted",
                stats.subtasks_accepted.global,
                stats.subtasks_accepted.session,
                "",
            ]),
            serde_json::json!([
                "computed",
                stats.subtasks_computed.global,
                stats.subtasks_computed.session,
                "",
            ]),
            serde_json::json!([
                "rejected",
                stats.subtasks_rejected.global,
                stats.subtasks_rejected.session,
                "",
            ]),
            serde_json::json!([
                "failed",
                stats.subtasks_with_errors.global,
                stats.subtasks_with_errors.session,
                "",
            ]),
            serde_json::json!([
                "timedout",
                stats.subtasks_with_timeout.global,
                stats.subtasks_with_timeout.session,
                "",
            ]),
        ];
        // CommandResponse::object(stats)
        Ok(ResponseTable { columns, values }.into())
    }

    async fn unsupport(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        last_days: &Option<i32>,
    ) -> Fallible<CommandResponse> {
        let unsupported = endpoint
            .as_golem_comp()
            .get_tasks_unsupported(last_days.unwrap_or(0).clone())
            .await?;

        let columns = vec![
            "reason".into(),
            "no of tasks".into(),
            "avg for all tasks".into(),
        ];
        let values = unsupported
            .into_iter()
            .map(|stat| serde_json::json!([stat.reason, stat.n_tasks, stat.avg,]))
            .collect();
        Ok(ResponseTable { columns, values }.into())
    }
}

fn seconds_to_human(time_remaining: f64) -> String {
    format_duration(Duration::new(
        time_remaining as u64,
        /*(time_remaining.fract() * 1_000_000_000.0) as u32*/ 0,
    ))
    .to_string()
}

fn fraction_to_percent(progress: f64) -> String {
    format!("{:.1} %", (progress * 100.0))
}

impl SubtaskCommand {
    async fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> failure::Fallible<CommandResponse> {
        match self {
            SubtaskCommand::List { task_id } => list_subtasks(endpoint, task_id.into()).await,
            SubtaskCommand::Show { subtask_id } => show_subtask(endpoint, subtask_id.into()).await,
            SubtaskCommand::Restart {
                task_id,
                subtask_ids,
            } => restart_subtasks(endpoint, task_id, subtask_ids).await,
        }
    }
}

async fn show_subtask<'a>(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    subtask_id: String,
) -> failure::Fallible<CommandResponse> {
    let (subtask, err_msg) = endpoint
        .as_golem_comp()
        .get_subtask(subtask_id.into())
        .await?;

    if subtask.is_some() {
        CommandResponse::object(subtask)
    } else {
        CommandResponse::object(err_msg)
    }
}

async fn list_subtasks(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    task_id: String,
) -> failure::Fallible<CommandResponse> {
    let subtasks = endpoint.as_golem_comp().get_subtasks(task_id).await?;

    if let Some(subtasks) = subtasks {
        let columns = vec![
            "node".into(),
            "subtask id".into(),
            "status".into(),
            "progress".into(),
        ];
        let values = subtasks
            .into_iter()
            .map(|subtask| {
                serde_json::json!([
                    subtask.node_name,
                    subtask.subtask_id,
                    subtask.status,
                    subtask.progress.map(fraction_to_percent),
                ])
            })
            .collect();
        Ok(ResponseTable { columns, values }.into())
    } else {
        CommandResponse::object("No subtasks")
    }
}

async fn restart_subtasks(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    task_id: &str,
    subtasks_ids: &Vec<String>,
) -> failure::Fallible<CommandResponse> {
    match endpoint
        .as_golem_comp()
        .restart_subtasks_from_task(task_id.into(), subtasks_ids.clone())
        .await?
    {
        serde_json::Value::Null => CommandResponse::object("Completed"),
        serde_json::Value::String(err_msg) => Err(failure::err_msg(err_msg)),
        serde_json::Value::Object(err) => CommandResponse::object(err),
        err => CommandResponse::object(err),
    }
}

// TODO: read it though rpc; requires exposing such RPC from Brass
pub async fn template(task_type: &str) -> Fallible<CommandResponse> {
    let template = match task_type {
        "blender" => serde_json::to_string_pretty(&golem_rpc_api::apps::blender::template())?,
        "wasm" => serde_json::to_string_pretty(&golem_rpc_api::apps::wasm::template())?,
        "glambda" => serde_json::to_string_pretty(&golem_rpc_api::apps::glambda::template())?,
        _ => failure::bail!("Invalid Option"),
    };
    CommandResponse::object(template)
}
