use crate::rpc::*;

rpc_interface! {

    trait GolemConcent {
        /// Returns true if concent is on
        #[id = "golem.concent.switch"]
        fn is_on(&self) -> Result<bool>;

        /// Turns concent on/off
        #[id = "golem.concent.switch.turn"]
        fn turn(&self, on : bool) -> Result<()>;

        #[id = "golem.concent.terms"]
        fn is_terms_accepted(&self) -> Result<bool>;

        #[id = "golem.concent.terms.accept"]
        fn accept_terms(&self) -> Result<()>;

        #[id = "golem.concent.terms.show"]
        fn show_terms(&self) -> Result<String>;

    }

    converter AsGolemConcent as_golem_concent;
}
