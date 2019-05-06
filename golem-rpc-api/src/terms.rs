#![allow(clippy::needless_lifetimes)]

use crate::rpc::*;
use serde_derive::*;

rpc_interface! {
    trait GolemTerms {
        #[id = "golem.terms"]
        fn are_terms_accepted(&self) -> Result<bool>;

        #[id = "golem.terms.accept"]
        fn accept_terms(&self, enable_monitor: Option<bool>, enable_talkback: Option<bool>) -> Result<()>;

        #[id= "golem.terms.show"]
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
