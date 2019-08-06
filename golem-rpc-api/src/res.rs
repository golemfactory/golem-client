use crate::rpc::*;
use serde::*;
use serde_json::Value;
use serde_repr::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

rpc_interface! {
    trait GolemRes {

        #[id = "res.dirs"]
        fn get_res_dirs(&self) -> Result<CachePaths>;

        #[id = "res.dirs.size"]
        fn get_res_dirs_sizes(&self) -> Result<CacheSizes>;

        //
        #[id = "res.dir"]
        fn get_res_dir(&self, dir_type : DirType) -> Result<Value>;

        #[id = "res.dir.clear"]
        fn clear_dir(&self, dir_type : DirType, #[kwarg] older_than_seconds : Option<usize>) -> Result<()>;

        #[id = "env.hw.caps"]
        fn get_hw_caps(&self) -> Result<HwCaps>;

        #[id = "env.hw.preset"]
        fn get_hw_preset(&self, name : String) -> Result<HwPreset>;

        #[id = "env.hw.presets"]
        fn get_hw_presets(&self) -> Result<Vec<HwPreset>>;

        #[id = "env.hw.preset.create"]
        fn create_hw_preset(&self, preset : HwPreset) -> Result<HwPreset>;

        #[id = "env.hw.preset.update"]
        fn update_hw_preset(&self, preset_update : HwPresetUpdate) -> Result<HwPreset>;

        #[id = "env.hw.preset.delete"]
        fn delete_hw_preset(&self, name : String) -> Result<bool>;

        #[id = "env.hw.preset.activate"]
        fn activate_hw_preset(&self, name : String, run_benchmarks : bool) -> Result<Option<BTreeMap<String, f64>>>;

    }

    converter AsGolemRes as_golem_res;
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
pub enum DirType {
    Distributed = 1,
    Received = 2,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachePaths {
    #[serde(rename = "total received data")]
    pub received_files: PathBuf,
    #[serde(rename = "total distributed data")]
    pub distributed_files: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheSizes {
    #[serde(rename = "total received data")]
    pub received_files: String,
    #[serde(rename = "total distributed data")]
    pub distributed_files: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HwCaps {
    pub cpu_cores: u32,
    /// disk in Kb
    pub disk: f64,
    /// memory in kb
    pub memory: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HwPreset {
    #[serde(flatten)]
    pub caps: HwCaps,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HwPresetUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_cores: Option<u32>,
    /// disk in Kb
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk: Option<f64>,
    /// memory in kb
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<u64>,

    pub name: String,
}
