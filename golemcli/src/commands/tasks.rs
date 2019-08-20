use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::future::Err;
use futures::prelude::*;
use humantime::format_duration;
use openssl::rsa::Padding;
use structopt::StructOpt;

use golem_rpc_api::comp::{AsGolemComp, StatsCounters, SubtaskStats, TaskInfo};
use serde_json::json;

use crate::context::*;

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
    },
    /// dry-run creating a task and return task dict
    #[structopt(name = "create_dry_run")]
    CreateDryRun {
        /// Task file
        file_name: PathBuf,
        ///  Output file
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
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            Section::Abort { task_id } => Box::new(self.abort(endpoint, task_id)),
            Section::Create { file_name } => Box::new(self.create(endpoint, file_name)),
            Section::CreateDryRun {
                file_name,
                out_file,
            } => Box::new(self.create_dry_run(endpoint, file_name, out_file)),
            Section::Delete { task_id } => Box::new(self.delete(endpoint, task_id)),
            Section::Dump { task_id, out_file } => Box::new(self.dump(endpoint, task_id, out_file)),
            Section::Purge => Box::new(self.purge(endpoint)),
            Section::Restart { task_id } => Box::new(self.restart(endpoint, task_id)),
            Section::List { current, sort } => Box::new(self.show(endpoint, &None, *current, sort)),
            Section::Show {
                task_id,
                current,
                sort,
            } => Box::new(self.show(endpoint, task_id, *current, sort)),
            Section::Template { task_type } => Box::new(template(task_type)),
            Section::Stats => Box::new(self.stats(endpoint)),
            Section::Subtasks(subtask_command) => subtask_command.run(endpoint),

            Section::Unsupport { last_days } => Box::new(self.unsupport(endpoint, last_days)),
        }
    }

    fn create(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        file_name: &Path,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        use std::fs;
        /*
         if error:
                    if isinstance(error, dict):
                        error = error['error_msg']
                    if task_id:
                        return CommandResult(error="task {} failed: {}"
                                             .format(task_id, error))
                    return CommandResult(error=error)
                return task_id
        */
        fs::OpenOptions::new()
            .read(true)
            .open(file_name)
            .into_future()
            .and_then(|file| Ok(serde_json::from_reader(file)?))
            .from_err()
            .and_then(move |task_spec| endpoint.as_golem_comp().create_task(task_spec).from_err())
            .and_then(|task_id| CommandResponse::object(task_id))
    }

    fn create_dry_run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        file_name: &Path,
        out_file: &Option<PathBuf>,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        use std::fs;
        let out_file = out_file.clone();

        fs::OpenOptions::new()
            .read(true)
            .open(file_name)
            .into_future()
            .and_then(|file| Ok(serde_json::from_reader(file)?))
            .from_err()
            .and_then(move |task_spec| {
                endpoint
                    .as_golem_comp()
                    .create_dry_run(task_spec)
                    .from_err()
            })
            .and_then(move |v| {
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
            })
    }

    fn abort(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_id: &str,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .abort_task(task_id.into())
            .from_err()
            .and_then(|()| CommandResponse::object("Completed"))
    }

    fn delete(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_id: &str,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .delete_task(task_id.into())
            .from_err()
            .and_then(|()| CommandResponse::object("Completed"))
    }

    fn purge(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .purge_tasks()
            .from_err()
            .and_then(|()| CommandResponse::object("Completed"))
    }

    fn restart(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_id: &str,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .restart_task(task_id.into())
            .from_err()
            .and_then(|(task_id, err_msg)| CommandResponse::object(task_id.or(err_msg)))
    }

    fn dump(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_id: &str,
        out_file: &Option<PathBuf>,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let out_file = out_file.clone();

        endpoint
            .as_golem_comp()
            .get_task(task_id.into())
            .from_err()
            .and_then(move |v| {
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
            })
    }

    fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + 'static,
        opt_task_id: &Option<String>,
        _current: bool,
        sort: &Option<String>,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let sort = sort.clone();

        if let Some(task_id) = opt_task_id {
            futures::future::Either::A(
                endpoint
                    .as_golem_comp()
                    .get_task(task_id.clone())
                    .from_err()
                    .and_then(|task| CommandResponse::object(task)),
            )
        } else {
            // TODO: filter for current
            futures::future::Either::B(endpoint.as_golem_comp().get_tasks().from_err().and_then(
                move |tasks: Vec<TaskInfo>| {
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
                },
            ))
        }
    }

    fn stats(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .get_tasks_stats()
            .from_err()
            .and_then(|stats: SubtaskStats| {
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
            })
    }

    fn unsupport(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        last_days: &Option<i32>,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .get_tasks_unsupported(last_days.unwrap_or(0).clone())
            .from_err()
            .and_then(|unsupported| {
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
            })
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
    fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            SubtaskCommand::List { task_id } => Box::new(list_subtasks(endpoint, task_id.into())),
            SubtaskCommand::Show { subtask_id } => {
                Box::new(show_subtask(endpoint, subtask_id.into()))
            }
            SubtaskCommand::Restart {
                task_id,
                subtask_ids,
            } => Box::new(restart_subtasks(endpoint, task_id, subtask_ids)),
        }
    }
}

fn show_subtask<'a>(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    subtask_id: String,
) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
    endpoint
        .as_golem_comp()
        .get_subtask(subtask_id.into())
        .from_err()
        .and_then(|(subtask, err_msg)| {
            subtask.map_or(CommandResponse::object(err_msg), CommandResponse::object)
        })
}

fn list_subtasks(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    task_id: String,
) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
    endpoint
        .as_golem_comp()
        .get_subtasks(task_id)
        .from_err()
        .and_then(move |subtasks| {
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
        })
}

fn restart_subtasks(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    task_id: &str,
    subtasks_ids: &Vec<String>,
) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
    endpoint
        .as_golem_comp()
        .restart_subtasks_from_task(task_id.into(), subtasks_ids.clone())
        .from_err()
        .and_then(|r: serde_json::Value| match r {
            serde_json::Value::Null => CommandResponse::object("Completed"),
            serde_json::Value::String(err_msg) => Err(failure::err_msg(err_msg)),
            serde_json::Value::Object(err) => CommandResponse::object(err),
            err => CommandResponse::object(err),
        })
}

// TODO: read it though rpc; requires exposing such RPC from Brass
pub fn template(task_type: &str) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
    (|| -> Result<CommandResponse, Error> {
        let template = match task_type {
            "blender" => serde_json::to_string_pretty(&golem_rpc_api::apps::blender::template())?,
            "wasm" => serde_json::to_string_pretty(&golem_rpc_api::apps::wasm::template())?,
            "glambda" => serde_json::to_string_pretty(&golem_rpc_api::apps::glambda::template())?,
            _ => failure::bail!("Invalid Option"),
        };
        CommandResponse::object(template)
    })()
    .into_future()
}
