use crate::context::*;
use actix::prelude::*;
use futures::prelude::*;
use golem_rpc_api::settings::DynamicSetting;
use golem_rpc_api::{core::AsGolemCore, settings, Map};
use serde_json::Value;
use std::collections::btree_map::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::io::{self, Write, StdoutLock};
use structopt::{clap, StructOpt};
use golem_rpc_api::comp::{AsGolemComp, SubtaskStats};
use golem_rpc_api::net::{AsGolemNet, NetStatus, PeerInfo};
use golem_rpc_api::core::ServerStatus;
use ansi_term::Colour::{Red, Green};
use ansi_term::Style;


#[derive(StructOpt, Debug)]
pub enum Section {
    /// Change settings (unimplemented) TODO:
    #[structopt(name = "status")]
    Status {

    }
}

/*
macro_rules! map {
    {
        $()
        $($key:expr => $value:expr,)+ => { map!($($key => $value),+) };

    };
};
*/

#[derive(Debug)]
struct ComputationStatus {
    subtasks_accepted: u32,
    subtasks_rejected: u32,
    subtasks_failed: u32,
    subtasks_computed: u32,
    subtasks_in_network: u32,
    provider_state: Option<String>
}

#[derive(Debug)]
struct FormattedGeneralStatus {
    net_status: NetStatus,
    nodes_online: usize,
    computation_status: ComputationStatus
}

/*
pub struct NetStatus {
    pub listening: bool,
    pub connected: bool,
    pub port_statuses: Map<u16, String>,
    pub msg: String,
}*/


impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            &Section::Status {} => Box::new(self.status(endpoint))
        }
    }


    pub fn status(&self, endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static)
        -> impl Future<Item = CommandResponse, Error = Error> + 'static {

    let online_nodes = endpoint.as_golem_net().get_connected_peers();
        let connection_status = endpoint.as_golem_net().connection_status();
        let status = endpoint.as_golem().status();
        let task_stats = endpoint.as_golem_comp().get_tasks_stats();
        let x = online_nodes.from_err().join4(connection_status.from_err(), status.from_err(), task_stats.from_err());

        let s = x.map(|(connected_nodes, net_status, b, d)| {

            let x : Option<String> = d.provider_state.get("status").cloned();

            let computation_status = ComputationStatus{
                subtasks_accepted: d.subtasks_accepted.session,
                subtasks_rejected: d.subtasks_rejected.session,
                subtasks_failed: d.subtasks_with_errors.session,
                subtasks_computed: d.subtasks_computed.session,
                subtasks_in_network: d.in_network,
                provider_state: x
            };

            let status = FormattedGeneralStatus {
                net_status: net_status,
                nodes_online: connected_nodes.len(),
                computation_status: computation_status
            };
            CommandResponse::FormattedObject(Box::new(status))
        });

        s
    }
}

impl FormattedGeneralStatus {
    fn print_network_status(&self, out: &mut Box<io::Stdout>){
        write!(*out, "{}:\n\tConnected: {}\n\tNumber of nodes in the network: {}\n",
               Style::new().bold().underline().paint("Network"),
               if self.net_status.connected {Green.paint("ONLINE") } else {Red.paint("OFFLINE")},
               self.nodes_online);
    }
    fn print_tasks_status(&self, out: &mut Box<io::Stdout>){
        write!(*out, "{}:\n\tSubtasks accepted (in session): {}\n\t\
        Subtasks rejected (in session): {}\n\tSubtasks failed (in session): {}\n\tSubtasks computed (in session): {}\n\t\
        Subtasks in network: {}\n", Style::new().bold().underline().paint("Computation"),
        self.computation_status.subtasks_accepted, self.computation_status.subtasks_rejected,
               self.computation_status.subtasks_failed, self.computation_status.subtasks_computed, self.computation_status.subtasks_in_network);
//        self.computation_status.provider_state.unwrap_or(String::from("Unknown"))
    }
}

impl FormattedObject for FormattedGeneralStatus {
    fn to_json(&self) -> Result<Value, Error> {
        Ok(serde_json::from_str("{}").unwrap())
    }

    fn print(&self) -> Result<(), Error> {
        let mut stdout = Box::new(io::stdout());

        self.print_network_status(&mut stdout);
        self.print_tasks_status(&mut stdout);
        Ok(())
//        let mut stdout = stdout.lock(); // blokowac?
////        io::stdout().write();
//        let mut s = String::new();  // reserve, stream
    }
}
