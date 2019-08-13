use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::comp::{AsGolemComp, CompEnvStatus};
use structopt::StructOpt;
use futures::future::Either;

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
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            Section::Enable { name } => Box::new(self.enable(endpoint, name)),
            Section::Disable { name } => Box::new(self.disable(endpoint, name)),
            Section::Show => Box::new(show(endpoint)),
            Section::PerfMult => Box::new(perf_mult(endpoint)),
            Section::PerfMultSet { multiplier } => {
                Box::new(self.perf_mult_set(endpoint, *multiplier))
            }
            Section::Recount { name } => Box::new(self.recount(endpoint, name)),
        }
    }

    fn enable(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        name: &str,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .enable_environment(name.into())
            .from_err()
            .and_then(|msg: Option<String>| {
                if let Some(msg) = msg {
                    Either::B(CommandResponse::object(msg).into_future())
                } else {
                    Either::A(show(endpoint))
                }
            })
    }

    fn disable(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        name: &str,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .disable_environment(name.into())
            .from_err()
            .and_then(|msg: Option<String>| {
                if let Some(msg) = msg {
                    Either::B(CommandResponse::object(msg).into_future())
                } else {
                    Either::A(show(endpoint))
                }
            })
    }

    fn recount(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        name: &str,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .run_benchmark(name.into())
            .from_err()
            .and_then(|v| CommandResponse::object(v))
    }

    fn perf_mult_set(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        multiplier: f64,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_comp()
            .perf_mult_set(multiplier)
            .from_err()
            .and_then(|()| perf_mult(endpoint))
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
            CommandResponse::object(format!(
                "minimal performance multiplier is: {}",
                multiplier
            ))
        })
}
