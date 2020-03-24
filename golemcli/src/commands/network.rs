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
    pub async fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> failure::Fallible<CommandResponse> {
        match self {
            NetworkSection::Connect { ip, port } => {
                endpoint
                    .as_golem_net()
                    .connect((ip.to_ascii_lowercase(), *port))
                    .await?;
                CommandResponse::object("Command Send")
            }
            NetworkSection::Dht { sort, full } => self.dht(endpoint, sort, *full).await,
            NetworkSection::Show { sort, full } => self.show(endpoint, sort, *full).await,
            NetworkSection::Status => {
                CommandResponse::object(endpoint.as_golem_net().connection_status().await?.msg)
            }
        }
    }

    async fn dht(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        sort: &Option<String>,
        full: bool,
    ) -> failure::Fallible<CommandResponse> {
        Ok(
            format_nodes(endpoint.as_golem_net().get_known_peers().await?, full)?
                .sort_by(sort)
                .into(),
        )
    }

    async fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        sort: &Option<String>,
        full: bool,
    ) -> failure::Fallible<CommandResponse> {
        Ok(
            format_peers(endpoint.as_golem_net().get_connected_peers().await?, full)?
                .sort_by(sort)
                .into(),
        )
    }
}

fn format_nodes(
    peers: impl IntoIterator<Item = NodeInfo>,
    full: bool,
) -> Fallible<ResponseTable> {
    let columns = vec!["ip".into(), "port".into(), "id".into(), "name".into()];

    let values = peers
        .into_iter()
        .map(|p: NodeInfo| {
            let port = p.p2p_pub_port.unwrap_or(p.p2p_prv_port.unwrap_or(0));

            serde_json::json!([p.pub_addr, port, format_key(&p.key, full), p.node_name])
        })
        .collect::<Vec<_>>();

    Ok(ResponseTable { columns, values })
}

fn format_peers(
    peers: impl IntoIterator<Item = PeerInfo>,
    full: bool,
) -> Fallible<ResponseTable> {
    let columns = vec!["ip".into(), "port".into(), "id".into(), "name".into()];

    let values = peers
        .into_iter()
        .map(|p: PeerInfo| {
            serde_json::json!([p.address, p.port, format_key(&p.key_id, full), p.node_name])
        })
        .collect();

    Ok(ResponseTable { columns, values })
}
