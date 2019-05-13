//! Blender task definition.
//!
//!

use serde::{Deserialize, Serialize};

use super::{ComputeOn, TaskDef, TaskDefOptions};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlenderTaskOptions {
    pub resolution: (u32, u32),
    pub format: String,
    pub compositing: bool,
    pub samples: u32,
    pub frame_count: u32,
    pub output_path: String,
}

impl TaskDefOptions for BlenderTaskOptions {
    const TASK_TYPE: &'static str = "Blender";
}

pub type BlenderTaskDef = TaskDef<BlenderTaskOptions>;

pub fn template() -> BlenderTaskDef {
    TaskDef {
        task_type: BlenderTaskOptions::TASK_TYPE.to_string(),
        compute_on: ComputeOn::CPU,
        name: "simple blender".to_string(),
        timeout: Duration::from_secs(600),
        subtask_timeout: Duration::from_secs(400),
        bid: 0.1,
        resources: vec!["/Users/tworec/git/golem/gu-gateway/golem/gugateway/horse.blend".into()],
        concent_enabled: false,
        options: BlenderTaskOptions {
            resolution: (800, 600),
            format: "PNG".to_string(),
            compositing: false,
            samples: 0,
            frame_count: 0,
            output_path: "/tmp/blender-out/".into(),
        },
    }
}
