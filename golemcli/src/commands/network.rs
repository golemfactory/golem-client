use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum NetworkSection {
    /// Block provider
    #[structopt(name = "block")]
    Block {
        /// ID of a node
        node_id: String,
    },
    /// Connect to a node
    #[structopt(name = "connect")]
    Connect {
        /// Remote IP address
        ip: String,
        /// Remote TCP port
        port: u16,
    },
    /// Show known nodes
    #[structopt(name = "dht")]
    Dht,
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
    /// Show client status
    #[structopt(name = "status")]
    Status,
}
