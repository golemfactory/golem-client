use crate::context::*;
use failure::Fallible;
use futures::prelude::*;
use golem_rpc_api::res::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Show information on used resources
    #[structopt(name = "show")]
    Show,

    /// Clear provider and requestor cache files
    #[structopt(name = "clear")]
    Clear,
}

impl Section {
    pub async fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        match self {
            Section::Show => self.show(endpoint).await,
            Section::Clear => self.clear(endpoint).await,
        }
    }

    async fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        let (sizes, dirs) = future::try_join(
            endpoint.as_golem_res().get_res_dirs_sizes(),
            endpoint.as_golem_res().get_res_dirs(),
        )
        .await?;

        CommandResponse::object(serde_json::json!({
            "cache_dir": dirs.received_files,
            "size": sizes.received_files
        }))
    }

    async fn clear(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        let (sizes, dirs) = future::try_join(
            endpoint.as_golem_res().get_res_dirs_sizes(),
            endpoint.as_golem_res().get_res_dirs(),
        )
        .await?;
        endpoint
            .as_golem_res()
            .clear_dir(DirType::Distributed, None)
            .await?;

        let after_clean_size = endpoint.as_golem_res().get_res_dirs_sizes().await?;
        CommandResponse::object(serde_json::json!({
            "cache_dir": dirs.received_files,
            "before_clean_size": sizes.received_files,
            "size": after_clean_size.received_files
        }))
    }
}
