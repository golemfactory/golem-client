//! Glambda task definition.
//!
//!


use super::{ComputeOn, TaskDef, TaskDefOptions};
use serde::{Deserialize, Serialize};
use std::time::Duration;

type GLambdaTaskDef = TaskDef<GLambdaOptions>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GLambdaOptions {
    pub method: String,
    pub args: String,
    pub verification: String,
    pub outputs: Vec<String>,
}

impl TaskDefOptions for GLambdaOptions {
    const TASK_TYPE: &'static str = "GLambda";
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GLambdaVerification {
    #[serde(rename = "type")]
    verification_type: GLambdaVerificationType,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum GLambdaVerificationType {
    #[serde(rename = "None")]
    NoVerification,
    #[serde(rename = "External")]
    ExternallyVerified,
}

pub fn template() -> GLambdaTaskDef {
    TaskDef {
        task_type: GLambdaOptions::TASK_TYPE.to_string(),
        compute_on: ComputeOn::CPU,
        name: "simple glambda".to_string(),
        timeout: Duration::from_secs(600),
        subtask_timeout: Duration::from_secs(400),
        bid: 0.1,
        resources: vec![],
        concent_enabled: false,
        options: GLambdaOptions {
            method: "".to_string(),
            args: "".to_string(),
            verification: "".to_string(),
            outputs: vec![],
        },
    }
}
