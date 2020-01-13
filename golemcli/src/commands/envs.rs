use crate::context::*;
use futures::future::Either;
use futures::prelude::*;
use golem_rpc_api::comp::{AsGolemComp, CompEnvStatus};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Show environments
    #[structopt(name = "show")]
    Show,
    /// Enable environment
    #[structopt(name = "enable")]
    Enable {
        // Environment name
        name: String,
    },
    /// Disable environment
    #[structopt(name = "disable")]
    Disable {
        // Environment name
        name: String,
    },
    /// Recount performance for an environment
    #[structopt(name = "recount")]
    Recount {
        /// Environment name
        name: String,
    },
    /// Prints current performance multiplier
    #[structopt(name = "perf_mult")]
    PerfMult,

    /// Sets performance multiplier
    #[structopt(name = "perf_mult_set")]
    PerfMultSet {
        /// Multiplier; float value within range [0, 100]
        multiplier: f64,
    },
}

impl Section {
    pub async fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> failure::Fallible<CommandResponse> {
        match self {
            Section::Enable { name } => self.enable(endpoint, name).await,
            Section::Disable { name } => self.disable(endpoint, name).await,
            Section::Show => show(endpoint).await,
            Section::PerfMult => perf_mult(endpoint).await,
            Section::PerfMultSet { multiplier } => self.perf_mult_set(endpoint, *multiplier).await,
            Section::Recount { name } => self.recount(endpoint, name).await,
        }
    }

    async fn enable(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        name: &str,
    ) -> failure::Fallible<CommandResponse> {
        if let Some(msg) = endpoint
            .as_golem_comp()
            .enable_environment(name.into())
            .await?
        {
            CommandResponse::object(msg)
        } else {
            show(endpoint).await
        }
    }

    async fn disable(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        name: &str,
    ) -> failure::Fallible<CommandResponse> {
        if let Some(msg) = endpoint
            .as_golem_comp()
            .disable_environment(name.into())
            .await?
        {
            CommandResponse::object(msg)
        } else {
            show(endpoint).await
        }
    }

    async fn recount(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        name: &str,
    ) -> failure::Fallible<CommandResponse> {
        CommandResponse::object(endpoint.as_golem_comp().run_benchmark(name.into()).await?)
    }

    async fn perf_mult_set(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        multiplier: f64,
    ) -> failure::Fallible<CommandResponse> {
        endpoint.as_golem_comp().perf_mult_set(multiplier).await?;
        perf_mult(endpoint).await
    }
}

fn show(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
    endpoint
        .as_golem_comp()
        .get_environments()
        .from_err()
        .and_then(|envs: Vec<CompEnvStatus>| {
            let columns = vec![
                "name".into(),
                "supported".into(),
                "active".into(),
                "performance".into(),
                "min accept. perf.".into(),
                "description".into(),
            ];
            let values = envs
                .into_iter()
                .map(|e| {
                    serde_json::json!([
                        e.id,
                        e.supported,
                        e.accepted,
                        e.performance,
                        e.min_accepted,
                        e.description
                    ])
                })
                .collect();
            Ok(ResponseTable { columns, values }.into())
        })
}

fn perf_mult(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
    endpoint
        .as_golem_comp()
        .perf_mult()
        .from_err()
        .and_then(|multiplier| {
            CommandResponse::object(format!("minimal performance multiplier is: {}", multiplier))
        })
}
