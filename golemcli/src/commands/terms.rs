use crate::context::*;
use futures::{future, prelude::*};
use golem_rpc_api::terms::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Show terms of use
    #[structopt(name = "show")]
    Show,

    /// Accept terms of use
    #[structopt(name = "accept")]
    Accept,
}

impl Section {
    pub async fn run(
        &self,
        ctx: &CliCtx,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Result<CommandResponse, Error> {
        match self {
            Section::Show => {
                let html = endpoint.as_golem_terms().show_terms().await?;
                let text = html2text::from_read(std::io::Cursor::new(html), 78);
                CommandResponse::object(text)
            }
            Section::Accept => {
                let enable_monitor = ctx.prompt_for_acceptance(
                    "Enable monitor",
                    Some("monitor will be ENABLED"),
                    Some("monitor will be DISABLED"),
                );
                let enable_talkback = ctx.prompt_for_acceptance(
                    "Enable talkback",
                    Some("talkback will be ENABLED"),
                    Some("talkback will be DISABLED"),
                );

                endpoint
                    .as_golem_terms()
                    .accept_terms(Some(enable_monitor), Some(enable_talkback))
                    .await?;
                CommandResponse::object("Terms of use have been accepted.")
            }
        }
    }
}
