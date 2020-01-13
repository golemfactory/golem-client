use crate::context::*;
use failure::Fallible;
use futures::{future, prelude::*};
use golem_rpc_api::rpc::AsInvoker;
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

impl Section {
    pub async fn run(
        &self,
        _: &mut CliCtx,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        match self {
            Section::ExposedProcedures => Ok(CommandResponse::Object(
                endpoint
                    .as_invoker()
                    .rpc_call("sys.exposed_procedures", &())
                    .await?,
            )),
            Section::Rpc { uri, vargs } => Ok(CommandResponse::Object(
                endpoint
                    .as_invoker()
                    .rpc_va_call(uri.to_owned(), vargs)
                    .await?,
            )),
        }
    }
}
