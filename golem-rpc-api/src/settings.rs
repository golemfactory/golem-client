use crate::Error;
use bigdecimal::BigDecimal;
use golem_rpc_macros::gen_settings;
use serde_json::Value;
use std::fmt::Display;
use std::str::FromStr;

gen_settings! {

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
        #[check("v > 0")]
        opt_peer_num: usize,

        /// Send ping messages to peers
        #[flag]
        send_pings: bool,

        /// Interval between ping messages
        #[check("v > 0")]
        pings_interval: usize,

        /// Enable error reporting with talkback service
        #[flag]
        enable_talkback: bool,
    }

    struct Requestor {
        /// Minimal provider trust
        #[check("-1.0 <= v <= 1.0")]
        computing_trust: f64,

        /// Max GNT/h price (requestor)
        #[unit = "GNT"]
        max_price: BigDecimal,
    }

    struct Provider {
        /// Minimal requestor trust
        #[check("-1.0 <= v <= 1.0")]
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
        #[check("v >= 1")]
        num_cores: usize,
    }
}

pub trait Setting {
    type Item;
    const NAME: &'static str;
    const DESC: &'static str;

    fn to_value(item: &Self::Item) -> Value;

    fn from_value(val: &Value) -> Result<Self::Item, Error>;
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
    <S::Item as FromStr>::Err: Display,
    S::Item: Display,
{
    fn name(&self) -> &str {
        S::NAME
    }

    fn description(&self) -> &str {
        S::DESC
    }

    fn parse_from_str(&self, value: &str) -> Result<Value, Error> {
        Ok(S::to_value(
            &(value.parse().map_err(|e| Error::Other(format!("{}", e))))?,
        ))
    }

    fn display_value(&self, value: &Value) -> Result<String, Error> {
        Ok(format!("{}", S::from_value(value)?))
    }
}



#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_node_name() {

        eprintln!("name: {}", general::NodeName::NAME);
        eprintln!("desc: {}", general::NodeName::DESC);
        eprintln!("name: {}", general::GettingPeersInterval::NAME);
        eprintln!("desc: {}", general::GettingPeersInterval::DESC);
        eprintln!("desc: {}", from_name("computing_trust").unwrap().description());

        eprintln!("GENERAL");
        for it in general::list() {
            eprintln!("{}: {}", it.name(), it.description());
        }
        eprintln!("PROVIDER");
        for it in provider::list() {
            eprintln!("{}: {}", it.name(), it.description());
        }
        eprintln!("REQUESTOR");
        for it in requestor::list() {
            eprintln!("{}: {}", it.name(), it.description());
        }


    }

}
