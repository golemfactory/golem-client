//! WASM task definition.
//!
//!

use super::{ComputeOn, TaskDef, TaskDefOptions};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WasmOptions {
    pub js_name: String,
    pub wasm_name: String,
    pub input_dir: String,
    pub output_dir: String,
    pub subtasks: HashMap<String, SubtaskDef>,
}

impl TaskDefOptions for WasmOptions {
    const TASK_TYPE: &'static str = "WASM";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubtaskDef {
    pub exec_args: Vec<String>,
    pub output_file_paths: Vec<String>,
}

pub type WasmTaskDef = TaskDef<WasmOptions>;

pub fn template() -> WasmTaskDef {
    let mut task_def = TaskDef {
        task_type: WasmOptions::TASK_TYPE.to_string(),
        compute_on: ComputeOn::CPU,
        name: "simple wasm".to_string(),
        timeout: Duration::from_secs(600),
        subtask_timeout: Duration::from_secs(400),
        subtasks_count: 1,
        bid: 0.1,
        resources: vec![],
        concent_enabled: false,
        options: WasmOptions {
            js_name: "".to_string(),
            wasm_name: "".to_string(),
            input_dir: "".to_string(),
            output_dir: "".to_string(),
            subtasks: HashMap::new(),
        },
    };
    task_def.options.subtasks.insert(
        "subtask1".into(),
        SubtaskDef {
            exec_args: vec![],
            output_file_paths: vec![],
        },
    );

    task_def
}
