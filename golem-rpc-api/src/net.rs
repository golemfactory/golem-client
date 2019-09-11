use super::Map;
use crate::rpc::*;
use crate::serde::opt_ts_seconds;
use serde::*;
use std::net::IpAddr;

rpc_interface! {

    trait GolemNet {
        #[id = "net.ident"]
        fn get_node(&self) -> Result<NodeInfo>;

        #[id = "net.ident.key"]
        fn get_node_key(&self) -> Result<String>;

        #[id = "net.ident.name"]
        fn get_node_name(&self) -> Result<String>;

        #[id = "net.p2p.port"]
        fn get_p2p_port(&self) -> Result<u16>;

        #[id = "net.tasks.port"]
        fn get_task_server_port(&self) -> Result<u16>;

        #[id = "net.status"]
        fn connection_status(&self) -> Result<NetStatus>;

        /// Connect to specific node
        ///
        #[id = "net.peer.connect"]
        fn connect(&self, peer: (String, u16)) -> Result<()>;

        ///
        /// ## Params
        ///
        /// * timeout_seconds - (-1) for persistent disallow.
        ///
        /// Returns:
        ///
        /// * `(true, None)` - if node is successively blocked.
        /// * `(false, reason)` - on error
        ///
        #[id = "net.peer.block"]
        fn block_node(&self, node_id: String, timeout_seconds : i32) -> Result<(bool, Option<String>)>;

        #[id = "net.peer.block_ip"]
        fn block_ip(&self, ip_addr: IpAddr, timeout_seconds : i32) -> Result<()>;


        #[id="net.peer.allow_ip"]
        fn allow_ip(&self, ip : IpAddr, timeout_seconds : i32) -> Result<()>;

        #[id="net.peer.allow"]
        fn allow_node(&self, node_id : String, timeout_seconds : i32) -> Result<()>;

        #[id="net.peer.acl"]
        fn acl_status(&self) -> Result<AclStatus<String>>;

        #[id="net.peer.acl_ip"]
        fn acl_ip_status(&self) -> Result<AclStatus<IpAddr>>;

        #[id="net.peer.acl.new"]
        fn acl_setup(&self, default_rule : AclRule, exceptions : Vec<String>) -> Result<()>;

        #[id = "net.peers.known"]
        fn get_known_peers(&self) -> Result<Vec<NodeInfo>>;

        #[id = "net.peers.connected"]
        fn get_connected_peers(&self) -> Result<Vec<PeerInfo>>;

    }
}

pub trait AsGolemNet: wamp::RpcEndpoint {
    fn as_golem_net<'a>(&'a self) -> GolemNet<'a, Self>;
}

impl<Endpoint: wamp::RpcEndpoint> AsGolemNet for Endpoint {
    fn as_golem_net<'a>(&'a self) -> GolemNet<'a, Endpoint> {
        GolemNet(self.as_invoker())
    }
}

/*
{"node_name": "", "key": "b6178fcd50f429dcb89b68a6c8c7b92a527d93ba9db1d1649e71819ca3eb1ad4aa145b04dc3780274aeb64044353be481586bc0b1c76a76672c7bfaebf6dd370",
 "prv_port": 40103, "pub_port": 40103, "p2p_prv_port": 40102, "p2p_pub_port": 40102, "prv_addr": "10.30.8.179", "pub_addr": "5.226.70.4",
  "prv_addresses": ["10.30.8.179", "25.52.239.71", "172.17.0.1", "172.18.0.1", "172.19.0.1", "172.21.0.1"],
  "hyperdrive_prv_port": 3282, "hyperdrive_pub_port": 3282,
   "port_statuses": {40102: "timeout", 40103: "timeout", 3282: "timeout"},
   "nat_type": []}
*/

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeInfo {
    pub node_name: String,
    pub key: String,
    pub prv_port: u16,
    pub pub_port: Option<u16>,
    pub p2p_prv_port: u16,
    pub p2p_pub_port: Option<u16>,
    pub prv_addr: String,
    pub pub_addr: String,
    pub prv_addresses: Vec<String>,
    pub nat_type: Vec<String>, // - TODO enum
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerInfo {
    pub address: String,
    pub port: u16,
    pub verified: bool,
    pub degree: i64,
    pub key_id: String,
    pub node_name: String,
    pub node_info: NodeInfo,
    pub listen_port: u16,
    pub conn_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetStatus {
    pub listening: bool,
    pub connected: bool,
    pub port_statuses: Map<u16, String>,
    pub msg: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AclStatus<Identity> {
    pub default_rule: AclRule,
    pub rules: Vec<AclRuleItem<Identity>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AclRuleItem<Identity> {
    pub identity: Identity,
    pub node_name: String,
    pub rule: AclRule,
    #[serde(default)]
    #[serde(with = "opt_ts_seconds")]
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum AclRule {
    Allow,
    Deny,
}
