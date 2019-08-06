use super::Map;
use crate::rpc::*;
use bigdecimal::BigDecimal;
use serde::*;
use serde_json::Value;

rpc_interface! {

    trait GolemComp {
        //
        // map kwarg force
        // Returns:
        //   Some(task_id), None,
        //   None, Some(error_message)
        #[id = "comp.task.create"]
        fn create_task_int(&self, task_spec: serde_json::Value) -> Result<(Option<String>, Option<Value>)>;

        #[id = "comp.task"]
        fn get_task(&self, task_id : String) -> Result<Option<TaskInfo>>;

        //
        // *Implementation note*
        // uri comp.tasks has optional argument task_id. with task_id
        // it works as get_task. We do not need this variant.
        //
        #[id = "comp.tasks"]
        fn get_tasks(&self) -> Result<Vec<TaskInfo>>;

        /// Show statistics for unsupported tasks.
        ///
        /// # Arguments
        ///
        /// * `last_days` -  Number of last days to compute statistics on.
        ///
        /// # Returns
        ///
        /// Vec of UnsupportInfo. With stats for each reason.
        ///
        #[id = "comp.tasks.unsupport"]
        fn get_tasks_unsupported(&self, last_days: i32) -> Result<Vec<UnsupportInfo>>;

        /// Abort task with given id.
        ///
        /// # Arguments
        ///
        /// * `task_id` - Task id to abort.
        ///
        #[id = "comp.task.abort"]
        fn abort_task(&self, task_id : String) -> Result<()>;

        #[id = "comp.task.delete"]
        fn delete_task(&self, taks_id : String) -> Result<()>;

        #[id = "comp.task.subtask.restart"]
        fn restart_subtask(&self, subtask_id : String) -> Result<()>;

        #[id = "comp.task.subtask"]
        fn get_subtask(&self, subtask_id : String) -> Result<(Option<SubtaskInfo>, Option<String>)>;

        #[id = "comp.task.subtasks"]
        fn get_subtasks(&self, task_id : String) -> Result<Option<Vec<SubtaskInfo>>>;

        #[id = "comp.task.purge"]
        fn purge_tasks(&self) -> Result<()>;

        //
        // (new_task_id, None) on success; (None, error_message) on failure
        #[id = "comp.task.restart"]
        fn restart_task(&self, task_id: String) -> Result<(Option<String>, Option<String>)>;

        // TODO:
        #[id = "comp.task.subtasks.frame.restart"]
        fn restart_frame_subtasks(&self, task_id: String, frame: u32) -> Result<()>;

        /// Restarts a set of subtasks from the given task. If the specified task is
        ///  already finished, all failed subtasks will be restarted along with the
        ///  set provided as a parameter. Finished subtasks will have their results
        ///  copied over to the newly created task.
        ///
        /// ## Parameters
        ///
        ///  * `task_id`  the ID of the task which contains the given subtasks.
        ///  * `subtask_ids` the set of subtask IDs which should be restarted. If this is
        /// empty and the task is finished, all of the task's subtasks marked as failed will be
        /// restarted.
        ///  * `ignore_gas_price` if True, this will ignore long transaction time
        ///        errors and proceed with the restart.
        ///  * `disable_concent`  setting this flag to True will result in forcing
        ///       Concent to be disabled for the task. This only has effect when the task
        ///        is already finished and needs to be restarted.
        ///
        ///  ##Returns
        ///
        ///  In case of any errors, returns the representation of the error
        /// (either a string or a dict). Otherwise, returns None.
        ///
        #[id = "comp.task.subtasks.restart"]
        fn restart_subtasks_from_task(&self, task_id: String, subtask_ids: Vec<String>) -> Result<Value>;

        //
        #[id = "comp.tasks.check"]
        fn run_test_task(&self, task_spec: serde_json::Value) -> Result<bool>;

        #[id = "comp.task.test.status"]
        fn check_test_status(&self) -> Result<TaskTestResult>;

        /// Returns true if there was task to cancel
        #[id = "comp.tasks.check.abort"]
        fn abort_test_task(&self) -> Result<bool>;

        #[id = "comp.tasks.stats"]
        fn get_tasks_stats(&self) -> Result<SubtaskStats>;

        #[id = "comp.environments"]
        fn get_environments(&self) -> Result<Vec<CompEnvStatus>>;

        /// Enables enviroment
        /// Returns None or Error message.
        #[id = "comp.environment.enable"]
        fn enable_environment(&self, env_id : String) -> Result<Option<String>>;

        /// Enables enviroment
        /// Returns None or Error message.
        #[id = "comp.environment.disable"]
        fn disable_environment(&self, env_id : String) -> Result<Option<String>>;

        #[id = "comp.environment.benchmark"]
        fn run_benchmark(&self, env_id : String) -> Result<Value>;

        // timeout=3s
        #[id = "performance.multiplier.update"]
        fn perf_mult_set(&self, multiplier : f64) -> Result<()>;

        #[id = "performance.multiplier"]
        fn perf_mult(&self) -> Result<f64>;

    }
}

impl<'a, Inner: crate::rpc::wamp::RpcEndpoint + ?Sized + 'static> GolemComp<'a, Inner> {
    //
    // map kwarg force
    // Returns:
    //   Some(task_id), None,
    //   None, Some(error_message)
    pub fn create_task(
        &self,
        task_spec: serde_json::Value,
    ) -> impl Future<Item = String, Error = crate::Error> {
        fn map_to_error<F: FnOnce(&String) -> String>(
            err_obj: Value,
            format_msg: F,
        ) -> crate::Error {
            match err_obj {
                Value::String(err_msg) => crate::Error::Other(format_msg(&err_msg)),
                Value::Object(err_obj) => match err_obj.get("error_msg") {
                    Some(Value::String(err_msg)) => crate::Error::Other(format_msg(&err_msg)),
                    _ => crate::Error::Other(format!("invalid error response: {:?}", err_obj)),
                },
                _ => crate::Error::Other(format!("invalid error response: {:?}", err_obj)),
            }
        }

        self.create_task_int(task_spec)
            .from_err()
            .and_then(|r: (Option<String>, Option<Value>)| match r {
                (Some(task_id), Some(err_obj)) => Err(map_to_error(err_obj, |err_msg| {
                    format!("task {} failed: {}", task_id, err_msg)
                })),
                (Some(task_id), None) => Ok(task_id),
                (None, Some(err_obj)) => Err(map_to_error(err_obj, |err_msg| err_msg.to_string())),
                (None, None) => Err(crate::Error::Other(format!("invalid error response: null"))),
            })
    }
}

pub trait AsGolemComp: wamp::RpcEndpoint {
    fn as_golem_comp<'a>(&'a self) -> GolemComp<'a, Self>;
}

impl<Endpoint: wamp::RpcEndpoint> AsGolemComp for Endpoint {
    fn as_golem_comp<'a>(&'a self) -> GolemComp<'a, Endpoint> {
        GolemComp(self.as_invoker())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TaskTestStatus {
    Started,
    Success,
    Error,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskTestResult {
    // required
    pub status: TaskTestStatus,
    #[serde(default)]
    pub result: Value,
    #[serde(default)]
    pub estimated_memory: Option<f64>,
    #[serde(default)]
    pub time_spent: Option<f64>,
    // string, or array
    #[serde(default)]
    pub error: Value,
    // TODO: dict
    #[serde(default)]
    pub more: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TaskStatus {
    #[serde(rename = "Error creating")]
    ErrorCreating,
    #[serde(rename = "Not started")]
    NotStarted,
    #[serde(rename = "Creating the deposit")]
    CreatingDeposit,
    Sending,
    Creating,
    Waiting,
    Starting,
    Computing,
    Finished,
    Aborted,
    Timeout,
    #[serde(rename = "Restart")]
    Restarted,
}

impl TaskStatus {
    pub fn is_active(&self) -> bool {
        match self {
            TaskStatus::Sending
            | TaskStatus::Waiting
            | TaskStatus::Starting
            | TaskStatus::Computing => true,
            _ => false,
        }
    }

    pub fn is_completed(&self) -> bool {
        match self {
            TaskStatus::Finished
            | TaskStatus::Aborted
            | TaskStatus::Timeout
            | TaskStatus::Restarted => true,
            _ => false,
        }
    }

    pub fn is_preparing(&self) -> bool {
        match self {
            TaskStatus::NotStarted | TaskStatus::CreatingDeposit => true,
            _ => false,
        }
    }
}

// TODO: Add more fields
// TODO: Add generic deserialization to different task definition schemas.
#[derive(Serialize, Deserialize, Debug)]
pub struct TaskInfo {
    pub id: String,
    pub status: TaskStatus,
    /// Remaining time in seconds
    pub time_remaining: Option<f64>,
    pub subtasks_count: Option<u32>,
    pub progress: Option<f64>,

    pub cost: Option<BigDecimal>,
    pub fee: Option<BigDecimal>,
    pub estimated_cost: Option<BigDecimal>,
    pub estimated_fee: Option<BigDecimal>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SubtaskStatus {
    Starting,
    Downloading,
    Verifying,
    #[serde(rename = "Failed - Resent")]
    FailedResent,
    Finished,
    Failure,
    Restart,
    Cancelled,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubtaskInfo {
    pub subtask_id: String,
    pub node_id: String,
    pub node_name: String,
    pub status: SubtaskStatus,
    pub progress: Option<f64>,
    pub time_started: Option<f64>,
    pub results: Vec<String>,
    pub stderr: Option<String>,
    pub stdout: Option<String>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatsCounters {
    pub session: u32,
    pub global: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubtasksInfo {
    pub frames: Vec<u32>,
    pub outfilebasename: String,
    pub output_format: String,
    pub progress: f64,
    pub running_time_seconds: f64,
    pub scene_file: String,
    pub seconds_to_timeout: f64,
    pub start_task: u32,
    pub subtask_id: String,
    pub total_tasks: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProviderState {
    pub status: String,
    pub subtask: Option<SubtasksInfo>,
    pub environment: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubtaskStats {
    pub provider_state: ProviderState,
    #[serde(rename(serialize = "subtasks_in_network"))]
    pub in_network: u32,
    #[serde(rename(serialize = "subtasks_supported"))]
    pub supported: u32,
    pub subtasks_accepted: StatsCounters,
    pub subtasks_computed: StatsCounters,
    pub subtasks_rejected: StatsCounters,
    pub subtasks_with_errors: StatsCounters,
    pub subtasks_with_timeout: StatsCounters,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnsupportInfo {
    pub reason: String,
    #[serde(rename = "ntasks")]
    pub n_tasks: u32,
    /// avg (if available) is the current most
    ///  typical corresponding value.  For unsupport reason
    ///  MAX_PRICE avg is the average price of all tasks currently observed in
    ///  the network. For unsupport reason APP_VERSION avg is
    ///  the most popular app version of all tasks currently observed in the
    ///  network.
    pub avg: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompEnvStatus {
    pub id: String,
    pub supported: bool,
    pub accepted: bool,
    pub performance: Option<f64>,
    pub min_accepted: f64,
    pub description: String,
}
