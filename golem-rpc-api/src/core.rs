use super::Map;
use crate::rpc::*;
#[cfg(feature = "settings")]
use crate::settings::{DynamicSetting, Setting};
use serde::*;
use serde_json::Value;

rpc_interface! {

    trait GolemCore {
        /// Get Golem node settings described in appconfig.ini file
        #[rpc_uri = "env.opts"]
        fn get_settings(&self) -> Result<Map<String, Value>>;

        #[rpc_uri = "env.opt"]
        fn raw_get_setting(&self, key : String) -> Result<Value>;

        #[rpc_uri = "env.opt.update"]
        fn raw_update_setting(&self, key : String, value : Value) -> Result<()>;

        #[rpc_uri = "env.opts.update"]
        fn update_settings(&self, settings_dict : Map<String, Value>) -> Result<()>;

        #[rpc_uri = "env.datadir"]
        fn get_datadir(&self) -> Result<String>;

        #[rpc_uri = "golem.version"]
        fn get_version(&self) -> Result<String>;

        #[rpc_uri = "golem.password.key_exists"]
        fn key_exists(&self) -> Result<bool>;

        #[rpc_uri = "golem.password.set"]
        fn set_password(&self, password : String) -> Result<bool>;

        #[rpc_uri = "golem.password.unlocked"]
        fn is_account_unlocked(&self) -> Result<bool>;

        #[rpc_uri = "golem.mainnet"]
        fn is_mainnet(&self) -> Result<bool>;

        #[rpc_uri = "golem.status"]
        fn status(&self) -> Result<ServerStatus>;

    }
}

#[cfg(feature = "settings")]
impl<'a, Endpoint: wamp::RpcEndpoint + 'static> GolemCore<'a, Endpoint> {
    pub fn update_setting<S: Setting>(
        &self,
        value: impl AsRef<S::Item>,
    ) -> impl Future<Output = Result<(), super::Error>> {
        let value = match S::to_value(value.as_ref()) {
            Ok(value) => value,
            Err(e) => return future::err(e).right_future(),
        };

        self.raw_update_setting(S::NAME.to_string(), value)
            .map_err(From::from)
            .left_future()
    }

    pub fn update_setting_dyn(
        &self,
        setting: &dyn DynamicSetting,
        value: &str,
    ) -> impl Future<Output = Result<(), wamp::Error>> + 'static {
        let key = setting.name().into();
        let value = setting.parse_from_str(value).unwrap();

        self.raw_update_setting(key, value)
    }

    pub fn get_setting<S: Setting>(&self) -> impl Future<Output = Result<S::Item, wamp::Error>> {
        self.raw_get_setting(S::NAME.to_string()).and_then(|value| {
            async move {
                Ok(S::from_value(&value).map_err(|e| {
                    wamp::Error::ProtocolError(std::borrow::Cow::Owned(format!("{}", e)))
                })?)
            }
        })
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
