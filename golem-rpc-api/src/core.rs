use super::Map;
use crate::rpc::*;
use serde_derive::*;
use serde_json::Value;
#[cfg(feature="settings")]
use crate::settings::{Setting, DynamicSetting};

rpc_interface! {

    trait GolemCore {
        #[id = "env.opts"]
        fn get_settings(&self) -> Result<Map<String, Value>>;

        #[id = "env.opt"]
        fn get_setting(&self, key : String) -> Result<Value>;

        #[id = "env.opt.update"]
        fn raw_update_setting(&self, key : String, value : Value) -> Result<()>;

        #[id = "env.opts.update"]
        fn update_settings(&self, settings_dict : Map<String, Value>) -> Result<()>;

        #[id = "env.datadir"]
        fn get_datadir(&self) -> Result<String>;

        #[id = "golem.version"]
        fn get_version(&self) -> Result<String>;

        #[id = "golem.password.set"]
        fn set_password(&self, password : String) -> Result<bool>;

        #[id = "golem.password.unlocked"]
        fn is_account_unlocked(&self) -> Result<bool>;

        #[id = "golem.mainnet"]
        fn is_mainnet(&self) -> Result<bool>;

        #[id = "golem.status"]
        fn status(&self) -> Result<ServerStatus>;

    }
}

#[cfg(feature="settings")]
impl<'a, Endpoint : wamp::RpcEndpoint + 'static> GolemCore<'a, Endpoint> {

    pub fn update_setting<S : Setting>(&self, _setting : &S, value : impl AsRef<S::Item>) -> impl Future<Item=(), Error=wamp::Error> {

        self.raw_update_setting(S::NAME.to_string(), S::to_value(value.as_ref()))
    }

    pub fn update_setting_dyn(&self, setting : &dyn DynamicSetting, value : &str) -> impl Future<Item=(), Error=wamp::Error> + 'static {
        let key = setting.name().into();
        let value = setting.parse_from_str(value).unwrap();

        self.raw_update_setting(key, value)
    }


}

pub trait AsGolemCore: wamp::RpcEndpoint {
    fn as_golem<'a>(&'a self) -> GolemCore<'a, Self>;
}

impl<Endpoint: wamp::RpcEndpoint> AsGolemCore for Endpoint {
    fn as_golem<'a>(&'a self) -> GolemCore<'a, Endpoint> {
        GolemCore(self.as_invoker())
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Pre,
    Post,
    Warning,
    Exception,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ComponentReport(
    /// Method
    pub String,
    /// Stage
    pub Stage,
    /// Extra
    pub Value,
);

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct ServerStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<ComponentReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker: Option<ComponentReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hypervisor: Option<ComponentReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ethereum: Option<ComponentReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperdrive: Option<ComponentReport>,
}
