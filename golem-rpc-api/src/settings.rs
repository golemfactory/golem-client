use crate::Error;
use bigdecimal::BigDecimal;
use serde_json::Value;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Failure)]
pub enum SettingError {}

pub trait Setting {
    type Item;
    const NAME: &'static str;
    const DESC: &'static str;

    fn to_value(item: &Self::Item) -> Value;

    fn from_value(val: &Value) -> Result<Self::Item, SettingError>;
}

pub trait DynamicSetting {
    fn name(&self) -> &str;

    fn description(&self) -> &str;

    fn parse_from_str(&self, value: &str) -> Result<Value, Error>;

    fn display_value(&self, value: &Value) -> Result<String, Error>;
}

impl<S: Setting> DynamicSetting for S
where
    S::Item: FromStr,
    S::Item: Display,
{
    fn name(&self) -> &str {
        S::NAME
    }

    fn description(&self) -> &str {
        S::DESC
    }

    fn parse_from_str(&self, value: &str) -> Result<Value, Error> {
        Ok(S::to_value(item.parse()?))
    }

    fn display_value(&self, value: &Value) -> Result<String, Error> {
        Ok(format!("{}", S::from_value(value)?))
    }
}

macro_rules! settings {
    ($(
        struct $group:id {
            $(
                $(#[$meta:meta])*
                $id:id : $t:ty
            )*
        }
    )+) => {};
}

struct General {
    /// Node name
    node_name: String,

    /// Accept tasks
    #[flag]
    accept_tasks: bool,

    /// Interval between task requests
    #[unit = "s"]
    getting_tasks_interval: usize,

    /// Interval between peer requests
    #[unit = "s"]
    getting_peers_interval: usize,

    /// Task session timeout
    #[unit = "s"]
    task_session_timeout: usize,

    /// P2P session timeout
    #[unit = "s"]
    p2p_session_timeout: usize,

    /// Use IPv6
    #[flag]
    use_ipv6: bool,

    /// Number of peers to keep
    #[range(0..)]
    opt_peer_num: usize,

    /// Send ping messages to peers
    #[flag]
    send_pings: bool,

    /// Interval between ping messages
    #[range(0..)]
    pings_interval: usize,

    /// Enable error reporting with talkback service
    #[flag]
    enable_talkback: bool,
}

struct Requestor {
    /// Minimal provider trust
    #[range(= -1.0 .. =1.0)]
    computing_trust: f64,

    /// Max GNT/h price (requestor)
    #[unit = "GNT"]
    max_price: BigDecimal,
}

struct Provider {
    /// Minimal requestor trust
    #[range(=-1.0 .. =1.0)]
    requesting_trust: f64,

    /// Min GNT/h price (provider)
    #[unit = "GNT"]
    min_price: BigDecimal,

    /// Maximal resource size
    #[unit = "kB"]
    max_resource_size: usize,

    /// Max memory size
    #[unit = "kB"]
    max_memory_size: usize,

    /// Number of CPU cores to use
    #[range(=1..)]
    num_cores: usize,
}
