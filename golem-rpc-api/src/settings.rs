use crate::Error;
use bigdecimal::BigDecimal;
use golem_rpc_macros::gen_settings;
use num_traits::cast::FromPrimitive;
use serde_json::Value;
use std::any::{Any, TypeId};
use std::fmt::Display;
use std::str::FromStr;

gen_settings! {

    struct General {
        /// Node name
        #[restart_required]
        node_name: String,

        /// Accept tasks
        #[flag]
        accept_tasks: bool,

        /// Interval between task requests
        #[unit = "s"]
        getting_tasks_interval: f64,

        /// Interval between peer requests
        #[unit = "s"]
        getting_peers_interval: f64,

        /// Task session timeout
        #[unit = "s"]
        task_session_timeout: f64,

        /// P2P session timeout
        #[unit = "s"]
        p2p_session_timeout: f64,

        /// Use IPv6
        #[flag]
        #[restart_required]
        use_ipv6: bool,

        /// Use UPnP for port forwarding.
        use_upnp : bool,

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

        /// Enable reporting to golem monitor service.
        enable_monitor : bool,

        /// Enable resources cleaning
        cleaning_enabled : bool,
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
        max_resource_size: f64,

        /// Max memory size
        #[unit = "kB"]
        #[check("v >= 1048576")]
        max_memory_size: usize,

        /// Number of CPU cores to use
        #[check("v >= 1")]
        num_cores: usize,

        /// Interval between request task from network.
        task_request_interval : f64,
    }
}

pub trait Setting {
    type Item;
    const NAME: &'static str;
    const DESC: &'static str;
    const VALIDATION_DESC: &'static str;

    fn to_value(item: &Self::Item) -> Result<Value, Error>;

    fn from_value(val: &Value) -> Result<Self::Item, Error>;
}

pub trait DynamicSetting {
    fn name(&self) -> &str;

    fn description(&self) -> &str;

    fn validation_desc(&self) -> &str;

    fn parse_from_str(&self, value: &str) -> Result<Value, Error>;

    fn display_value(&self, value: &Value) -> Result<String, Error>;
}

impl<S: Setting> DynamicSetting for S
where
    S::Item: FromStr + Any,
    <S::Item as FromStr>::Err: Display,
    S::Item: Display,
{
    fn name(&self) -> &str {
        S::NAME
    }

    fn description(&self) -> &str {
        S::DESC
    }

    fn validation_desc(&self) -> &str {
        S::VALIDATION_DESC
    }

    // Very ugly part.
    fn parse_from_str(&self, value: &str) -> Result<Value, Error> {
        if TypeId::of::<S::Item>() == TypeId::of::<bool>() {
            let b = match value {
                "true" | "1" | "True" => Ok(true),
                "false" | "0" | "False" => Ok(false),
                _ => Err(Error::Other(format!("invalid flag: '{:?}'", value))),
            }?;
            Ok(S::to_value(Any::downcast_ref(&b).unwrap())?)
        } else {
            Ok(S::to_value(
                &(value.parse().map_err(|e| Error::Other(format!("{}", e))))?,
            )?)
        }
    }

    fn display_value(&self, value: &Value) -> Result<String, Error> {
        Ok(format!("{}", S::from_value(value)?))
    }
}

fn bool_from_value(value: &Value) -> Result<bool, Error> {
    (match value {
        Value::Bool(b) => Ok(*b),
        Value::Number(n) => match n.as_u64() {
            Some(1) => Ok(true),
            Some(0) => Ok(false),
            _ => Err(()),
        },
        Value::String(s) => match s.as_str() {
            "true" | "1" | "True" => Ok(true),
            "false" | "0" | "False" => Ok(false),
            _ => Err(()),
        },
        _ => Err(()),
    })
    .map_err(|()| Error::Other(format!("invalid bool: '{:?}'", value)))
}

fn bool_to_value(b: bool) -> Value {
    serde_json::json!(if b { 1 } else { 0 })
}

fn gnt_from_value(value: &Value) -> Result<bigdecimal::BigDecimal, Error> {
    let decimal: bigdecimal::BigDecimal = serde_json::from_value(value.clone())?;

    Ok(decimal / bigdecimal::BigDecimal::from_u128(1_000_000__000_000__000_000).unwrap())
}

fn gnt_to_value(gnt: &bigdecimal::BigDecimal) -> Result<Value, Error> {
    Ok(serde_json::to_value(
        (gnt * bigdecimal::BigDecimal::from_u128(1_000_000__000_000__000_000).unwrap())
            .with_scale(0),
    )?)
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
        eprintln!("vdesc {}", general::GettingPeersInterval::VALIDATION_DESC);
        eprintln!(
            "desc: {}",
            from_name("computing_trust").unwrap().description()
        );

        eprintln!("\nGENERAL\n");
        for it in general::list() {
            eprintln!(
                "{:30} {:50} {}",
                it.name(),
                it.description(),
                it.validation_desc()
            );
        }
        eprintln!("\nPROVIDER\n");
        for it in provider::list() {
            eprintln!(
                "{:30} {:50} {}",
                it.name(),
                it.description(),
                it.validation_desc()
            );
        }
        eprintln!("\nREQUESTOR\n");
        for it in requestor::list() {
            eprintln!(
                "{:30} {:50} {}",
                it.name(),
                it.description(),
                it.validation_desc()
            );
        }
    }

}
