use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::prelude::*;
use humantime::format_duration;
use openssl::rsa::Padding;
use structopt::StructOpt;

use golem_rpc_api::comp::{AsGolemComp, StatsCounters, SubtaskStats, TaskInfo};

use crate::context::*;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Abort a task
    #[structopt(name = "abort")]
    Abort {
        /// Task identifier
        id: String,
    },
    /// Create a task from file. Note: no client-side validation is performed yet.
    /// This will change in the future
    #[structopt(name = "create")]
    Create {
        /// Task file
        file_name: PathBuf,
    },
    /// Delete a task
    #[structopt(name = "delete")]
    Delete {
        /// Task identifier
        id: String,
    },
    /// Dump an existing task
    #[structopt(name = "dump")]
    Dump {
        /// Task identifier
        id: String,
        ///  Output file
        out_file: Option<PathBuf>,
    },
    /// Deletes all tasks
    #[structopt(name = "purge")]
    Purge,
    /// Restart a task
    #[structopt(name = "restart")]
    Restart {
        /// Task identifier
        id: String,
    },
    /// Restart given subtasks from a task
    #[structopt(name = "restart_subtasks")]
    RestartSubtasks {
        id: String,
        subtask_ids: Vec<String>,
    },
    /// Show task details
    #[structopt(name = "show")]
    Show {
        /// Task identifier
        id: Option<String>,
        /// Show only current tasks
        #[structopt(long)]
        current: bool,

        /// Sort tasks
        #[structopt(long)]
        sort: Option<String>,
    },
    /// Show statistics for tasks (unimplemented)
    #[structopt(name = "stats")]
    Stats,
    /// Show sub-tasks (unimplemented)
    #[structopt(name = "subtasks")]
    Subtasks {
        /// Task identifier
        task_id: String,
    },
    /// Dump a task template (unimplemented)
    #[structopt(name = "template")]
    Template,
    /// Show statistics for unsupported tasks (unimplemented)
    #[structopt(name = "unsupport")]
    Unsupport { last_days: Option<i32> },
}

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            Section::Abort { id } => Box::new(self.abort(endpoint, id)),
            Section::Create { file_name } => Box::new(self.create(endpoint, file_name)),
            Section::Delete { id } => Box::new(self.delete(endpoint, id)),
            Section::Dump { id, out_file } => Box::new(self.dump(endpoint, id, out_file)),
            Section::Purge => Box::new(self.purge(endpoint)),
            Section::Restart { id } => Box::new(self.restart(endpoint, id)),
            Section::RestartSubtasks { id, subtask_ids } => {
                Box::new(self.restart_subtasks(endpoint, id, subtask_ids))
            }
            Section::Show { id, current, sort } => {
                Box::new(self.show(endpoint, id, *current, sort))
            }
            Section::Template => Box::new(self.template()),
            Section::Stats => Box::new(self.stats(endpoint)),
            Section::Subtasks { task_id } => Box::new(self.subtasks(endpoint, task_id)),
            Section::Unsupport { last_days } => Box::new(self.unsupport(endpoint, last_days)),
            _ => Box::new(futures::future::err(unimplemented!())),
        }
    }

    fn create(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        file_name: &Path,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        use std::fs;

        fs::OpenOptions::new()
            .read(true)
            .open(file_name)
            .into_future()
            .and_then(|file| Ok(serde_json::from_reader(file)?))
            .from_err()
            .and_then(move |task_spec| endpoint.as_golem_comp().create_task(task_spec).from_err())
            .and_then(|(task_id, err_msg)| CommandResponse::object(task_id.or(err_msg)))
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

    fn restart_subtasks(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_id: &str,
        subtasks_ids: &Vec<String>,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .restart_subtasks_from_task(task_id.into(), subtasks_ids.clone())
            .from_err()
            .and_then(|()| CommandResponse::object("Completed"))
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

    // TODO: read it though rpc; requires exposing such RPC from Brass
    fn template(&self) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        futures::future::result(CommandResponse::object(
            r#"{
    "type": "Blender",
    "compute_on": "cpu",
    "name": "Horse 3s",
    "timeout": "00:15:00",
    "subtask_timeout": "00:10:00",
    "subtasks_count": 3,
    "bid": 3.3,
    "resources": [
        "/Users/tworec/git/golem/gu-gateway/golem/gugateway/horse.blend"
    ],
    "options": {
        "frame_count": 1,
        "output_path": "/Users/tworec/tmp/",

        "format": "PNG",
        "resolution": [
            1000,
            600
        ],
        "frames": "1",
        "compositing": false
    },
    "concent_enabled": false
}"#,
        ))
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

    fn subtasks(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        task_id: &str,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .get_subtasks(task_id.into())
            .from_err()
            .and_then(|subtasks| {
                let columns = vec![
                    "node".into(),
                    "subtask id".into(),
                    "ETA".into(),
                    "status".into(),
                    "progress".into(),
                ];
                let values = match subtasks {
                    Some(subtasks) => subtasks
                        .into_iter()
                        .map(|subtask| {
                            serde_json::json!([
                                subtask.node_name,
                                subtask.subtask_id,
                                subtask.time_remaining.map(seconds_to_human),
                                subtask.status,
                                subtask.progress.map(fraction_to_percent),
                            ])
                        })
                        .collect(),
                    None => vec![],
                };
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
                    .map(|stat| serde_json::json!([stat.reason, stat.ntasks, stat.avg,]))
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
