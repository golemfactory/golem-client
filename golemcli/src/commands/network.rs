use crate::context::CommandResponse::Object;
use crate::context::*;
use futures::{future, prelude::*};
use golem_rpc_api::net::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum NetworkSection {
    /// Show client status
    #[structopt(name = "status")]
    Status,
    /// Show connected nodes
    #[structopt(name = "show")]
    Show {
        /// Show full table contents
        #[structopt(long)]
        full: bool,

        /// Sort nodes
        /// ip, port, id, name
        #[structopt(long)]
        sort: Option<String>,
    },
    /// Show known nodes
    #[structopt(name = "dht")]
    Dht {
        /// Show full table contents
        #[structopt(long)]
        full: bool,

        /// Sort nodes
        /// ip, port, id, name
        #[structopt(long)]
        sort: Option<String>,
    },
    /// Connect to a node
    #[structopt(name = "connect")]
    Connect {
        /// Remote IP address
        ip: String,
        /// Remote TCP port
        port: u16,
    },
}

impl NetworkSection {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            NetworkSection::Connect { ip, port } => Box::new(
                endpoint
                    .as_golem_net()
                    .connect((ip.to_ascii_lowercase(), *port))
                    .from_err()
                    .and_then(|_| CommandResponse::object("Command Send")),
            ),
            NetworkSection::Dht { sort, full } => Box::new(self.dht(endpoint, sort, *full)),
            NetworkSection::Show { sort, full } => Box::new(self.show(endpoint, sort, *full)),
            NetworkSection::Status => Box::new(
                endpoint
                    .as_golem_net()
                    .connection_status()
                    .from_err()
                    .and_then(|status| CommandResponse::object(status.msg)),
            ),
        }
    }

    fn dht(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        sort: &Option<String>,
        full: bool,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let sort = sort.clone();
        endpoint
            .as_golem_net()
            .get_known_peers()
            .from_err()
            .and_then(move |peers| Ok(format_nodes(peers, full)?.sort_by(&sort).into()))
    }

    fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        sort: &Option<String>,
        full: bool,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let sort = sort.clone();
        endpoint
            .as_golem_net()
            .get_connected_peers()
            .from_err()
            .and_then(move |peers| Ok(format_peers(peers, full)?.sort_by(&sort).into()))
    }
}

fn format_nodes(
    peers: impl IntoIterator<Item = NodeInfo>,
    full: bool,
) -> Result<ResponseTable, Error> {
    let columns = vec!["ip".into(), "port".into(), "id".into(), "name".into()];

    let values = peers
        .into_iter()
        .map(|p: NodeInfo| {
            let port = p.p2p_pub_port.unwrap_or(p.p2p_prv_port);

            serde_json::json!([p.pub_addr, port, format_key(&p.key, full), p.node_name])
        })
        .collect();

    Ok(ResponseTable { columns, values })
}

fn format_peers(
    peers: impl IntoIterator<Item = PeerInfo>,
    full: bool,
) -> Result<ResponseTable, Error> {
    let columns = vec!["ip".into(), "port".into(), "id".into(), "name".into()];

    let values = peers
        .into_iter()
        .map(|p: PeerInfo| {
            serde_json::json!([p.address, p.port, format_key(&p.key_id, full), p.node_name])
        })
        .collect();

    Ok(ResponseTable { columns, values })
}
