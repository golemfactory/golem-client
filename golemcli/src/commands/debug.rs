use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Dump uri to procedure mapping
    #[structopt(name = "exposed_procedures")]
    ExposedProcedures,

    /// Debug RPC calls
    #[structopt(name = "rpc")]
    Rpc {
        /// Remote procedure uri
        uri: String,
        /// Call arguments
        vargs: Vec<String>,
    },
}
