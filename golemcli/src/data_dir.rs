use actix_wamp::Error;
use failure::Fallible;
use std::path::{Path, PathBuf};

const APP_NAME: &str = "golem";
const DEFAULT_DATA_DIR: &str = "default";

pub fn get_local_data(data_dir: &Path, env_type: &str) -> Fallible<PathBuf> {
    let data_dir = appdirs::user_data_dir(Some(APP_NAME), None, false)
        .map_err(|_| Error::protocol_err("unable to get user data dir"))?;

    Ok(data_dir)
}
