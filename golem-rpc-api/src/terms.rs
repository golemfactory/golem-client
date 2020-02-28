#![allow(clippy::needless_lifetimes)]

use crate::rpc::*;

rpc_interface! {
    trait GolemTerms {
        #[rpc_uri = "golem.terms"]
        fn are_terms_accepted(&self) -> Result<bool>;

        #[rpc_uri = "golem.terms.accept"]
        fn accept_terms(&self, enable_monitor: Option<bool>, enable_talkback: Option<bool>) -> Result<()>;

        #[rpc_uri= "golem.terms.show"]
        fn show_terms(&self) -> Result<String>;
    }
}

pub trait AsGolemTerms: wamp::RpcEndpoint {
    fn as_golem_terms<'a>(&'a self) -> GolemTerms<'a, Self>;
}

impl<Endpoint: wamp::RpcEndpoint> AsGolemTerms for Endpoint {
    fn as_golem_terms<'a>(&'a self) -> GolemTerms<'a, Endpoint> {
        GolemTerms(self.as_invoker())
    }
}
