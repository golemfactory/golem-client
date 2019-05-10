use serde::{Serialize, Deserialize};
use std::time::Duration;
use crate::serde::duration;

pub mod blender;
pub mod glambda;
pub mod wasm;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskDef<Options : TaskDefOptions> {
    #[serde(rename="type")]
    task_type : String,
    compute_on : ComputeOn,
    name : String,
    #[serde(with = "duration")]
    timeout : Duration,
    #[serde(with = "duration")]
    subtask_timeout : Duration,
    // GNT/h
    bid : f64,
    resources : Vec<String>,
    concent_enabled : bool,
    options : Options
}

pub trait TaskDefOptions {
    const TASK_TYPE : &'static str;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all="lowercase")]
pub enum ComputeOn {
    GPU,
    CPU
}




