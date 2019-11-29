use crate::rpc::*;

rpc_interface! {

    trait GolemConcent {
        /// Returns true if concent is on
        #[rpc_uri = "golem.concent.switch"]
        fn is_on(&self) -> Result<bool>;

        /// Turns concent on/off
        #[rpc_uri = "golem.concent.switch.turn"]
        fn turn(&self, on : bool) -> Result<()>;

        #[rpc_uri = "golem.concent.terms"]
        fn is_terms_accepted(&self) -> Result<bool>;

        #[rpc_uri = "golem.concent.terms.accept"]
        fn accept_terms(&self) -> Result<()>;

        #[rpc_uri = "golem.concent.terms.show"]
        fn show_terms(&self) -> Result<String>;

    }

    converter AsGolemConcent as_golem_concent;
}
