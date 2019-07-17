use super::Map;
use crate::rpc::*;
use bigdecimal::BigDecimal;
use serde_derive::*;
use serde_json::Value;

rpc_interface! {

    trait GolemComp {
        //
        // map kwarg force
        // Returns:
        //   Some(task_id), None,
        //   None, Some(error_message)
        #[id = "comp.task.create"]
        fn create_task(&self, task_spec: serde_json::Value) -> Result<(Option<String>, Option<String>)>;

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

        //
        #[id = "comp.task.subtasks.frame.restart"]
        fn restart_frame_subtasks(&self, task_id: String, frame: u32) -> Result<()>;

        //
        #[id = "comp.task.restart_subtasks"]
        fn restart_subtasks_from_task(&self, task_id: String, subtask_ids: Vec<String>) -> Result<()>;

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

#[derive(Serialize, Deserialize, Debug)]
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
pub struct SubtaskStats {
    pub provider_state: Map<String, String>,
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
    #[serde(rename(serialize = "no_of_tasks"))]
    pub ntasks: u32,
    #[serde(rename(serialize = "avg_for_all_tasks"))]
    pub avg: Option<f32>,
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
