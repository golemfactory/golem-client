use super::{Map, Result};
use serde_derive::*;
use wamp_derive::*;

#[wamp_interface]
pub trait GolemNet {
    #[wamp(id = "net.ident")]
    fn get_node(&mut self) -> Result<NodeInfo>;

    #[wamp(id = "net.ident.key")]
    fn get_node_key(&mut self) -> Result<String>;

    #[wamp(id = "net.ident.name")]
    fn get_node_name(&mut self) -> Result<String>;

    #[wamp(id = "net.p2p.port")]
    fn get_p2p_port(&mut self) -> Result<u16>;

    #[wamp(id = "net.tasks.port")]
    fn get_task_server_port(&mut self) -> Result<u16>;

    #[wamp(id = "net.status")]
    fn connection_status(&mut self) -> Result<NetStatus>;

    /// Connect to specific node
    ///
    #[wamp(id = "net.peer.connect")]
    fn connect(&mut self, peer: (String, u16)) -> Result<()>;

    ///
    /// Returns:
    ///
    ///    (true, None) - if node is successively blocked.
    ///    (false, reason) - on error
    ///
    #[wamp(id = "net.peer.block")]
    fn block_node(&mut self, node_id: String) -> Result<(bool, String)>;

    #[wamp(id = "net.peers.known")]
    fn get_known_peers(&mut self) -> Result<Vec<PeerInfo>>;

    #[wamp(id = "net.peers.connected'")]
    fn get_connected_peers(&mut self) -> Result<Vec<PeerInfo>>;
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
    pub pub_port: u16,
    pub p2p_prv_port: u16,
    pub p2p_pub_port: u16,
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
    pub conn_id: i128,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetStatus {
    pub listening: bool,
    pub connected: bool,
    pub port_statuses: Map<u16, String>,
    pub msg: String,
}
