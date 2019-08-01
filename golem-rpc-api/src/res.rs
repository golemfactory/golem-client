use crate::rpc::*;
use serde::*;
use serde_json::Value;
use serde_repr::*;
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
