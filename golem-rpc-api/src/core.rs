use super::Map;
use crate::rpc::*;
#[cfg(feature = "settings")]
use crate::settings::{DynamicSetting, Setting};
use serde_derive::*;
use serde_json::Value;

rpc_interface! {

    trait GolemCore {
        /// Get Golem node settings described in appconfig.ini file
        #[id = "env.opts"]
        fn get_settings(&self) -> Result<Map<String, Value>>;

        #[id = "env.opt"]
        fn raw_get_setting(&self, key : String) -> Result<Value>;

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

#[cfg(feature = "settings")]
impl<'a, Endpoint: wamp::RpcEndpoint + 'static> GolemCore<'a, Endpoint> {
    pub fn update_setting<S: Setting>(
        &self,
        value: impl AsRef<S::Item>,
    ) -> impl Future<Item = (), Error = super::Error> {
        let value = match S::to_value(value.as_ref()) {
            Ok(value) => value,
            Err(e) => return future::Either::B(future::err(e)),
        };

        future::Either::A(
            self.raw_update_setting(S::NAME.to_string(), value)
                .from_err(),
        )
    }

    pub fn update_setting_dyn(
        &self,
        setting: &dyn DynamicSetting,
        value: &str,
    ) -> impl Future<Item = (), Error = wamp::Error> + 'static {
        let key = setting.name().into();
        let value = setting.parse_from_str(value).unwrap();

        self.raw_update_setting(key, value)
    }

    pub fn get_setting<S: Setting>(&self) -> impl Future<Item = S::Item, Error = wamp::Error> {
        self.raw_get_setting(S::NAME.to_string()).and_then(|value| {
            Ok(S::from_value(&value).map_err(|e| {
                wamp::Error::ProtocolError(std::borrow::Cow::Owned(format!("{}", e)))
            })?)
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
