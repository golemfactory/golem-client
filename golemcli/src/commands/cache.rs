use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::res::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Clear provider / requestor resources
    #[structopt(name = "clear")]
    Clear,
    /// Show information on used resources
    #[structopt(name = "show")]
    Show,
}

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            Section::Show => Box::new(self.show(endpoint)),
            Section::Clear => Box::new(self.clear(endpoint)),
        }
    }

    fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_res()
            .get_res_dirs_sizes()
            .join(endpoint.as_golem_res().get_res_dirs())
            .from_err()
            .and_then(|(sizes, dirs)| {
                CommandResponse::object(serde_json::json!({
                    "cache_dir": dirs.received_files,
                    "size": sizes.received_files
                }))
            })
    }

    fn clear(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_res()
            .get_res_dirs_sizes()
            .join(endpoint.as_golem_res().get_res_dirs())
            .from_err()
            .and_then(move |(sizes, dirs)| {
                endpoint
                    .as_golem_res()
                    .clear_dir(DirType::Distributed, None)
                    .from_err()
                    .and_then(move |()| {
                        endpoint
                            .as_golem_res()
                            .get_res_dirs_sizes()
                            .from_err()
                            .and_then(move |after_clean_size| {
                                CommandResponse::object(serde_json::json!({
                                "cache_dir": dirs.received_files,
                                "before_clean_size": sizes.received_files,
                                "size": after_clean_size.received_files}))
                            })
                    })
            })
    }
}
