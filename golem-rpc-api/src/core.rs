use crate::rpc::*;
use serde_derive::*;
use super::Map;
use serde_json::Value;

rpc_interface! {

    trait GolemCore {
        #[id = "env.opts"]
        fn get_settings(&self) -> Result<Map<String, Value>>;

        #[id = "env.opt"]
        fn get_setting(&self, key : String) -> Result<Value>;

        #[id = "env.opt.update"]
        fn update_setting(&self, key : String, value : Value) -> Result<()>;

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

pub trait AsGolemCore: wamp::RpcEndpoint {
    fn as_golem<'a>(&'a self) -> GolemCore<'a, Self>;
}

impl<Endpoint: wamp::RpcEndpoint> AsGolemCore for Endpoint {
    fn as_golem<'a>(&'a self) -> GolemCore<'a, Endpoint> {
        GolemCore(self.as_invoker())
    }
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all="lowercase")]
pub enum Stage {
    Pre,
    Post,
    Warning,
    Exception
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ComponentReport(
    /// Method
    pub String,
    /// Stage
    pub Stage,
    /// Extra
    pub Value
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
    pub hyperdrive: Option<ComponentReport>
}
