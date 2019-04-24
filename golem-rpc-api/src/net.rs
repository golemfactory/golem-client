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
    pub nat_type: String, // - TODO enum
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
