use crate::context::*;
use actix::prelude::*;
use futures::prelude::*;
use golem_rpc_api::settings::DynamicSetting;
use golem_rpc_api::{core::AsGolemCore, settings, Map, pay::AsGolemPay};
use serde_json::Value;
use std::collections::btree_map::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::io::{self, Write, StdoutLock};
use futures::future::{Join, Future, ok};
use structopt::{clap, StructOpt};
use golem_rpc_api::comp::{AsGolemComp, SubtaskStats};
use golem_rpc_api::net::{AsGolemNet, NetStatus, PeerInfo, NodeInfo};
use golem_rpc_api::core::ServerStatus;
use ansi_term::Colour::{Red, Green};
use ansi_term::Style;
use crate::component_response::map_statuses;
use golem_rpc_api::rpc::AsInvoker;
use golem_rpc_api::pay::{PaymentStatus, Balance};
use bigdecimal::{Zero, BigDecimal};
use std::mem;


#[derive(StructOpt, Debug)]
pub enum Section {
    /// Change settings (unimplemented) TODO:
    #[structopt(name = "status")]
    Status {

    }
}

#[derive(Debug)]
#[derive(PartialEq)] // ok?
enum ProcessState {
    Running,
    Stopped,
    Stopping
}

#[derive(Debug)]
#[derive(PartialEq)] // ok?
// implement to string
enum GolemNet {
    Mainnet,
    Testnet
}

#[derive(Debug)]
struct ComponentStatuses {
    docker_status: Option<String>,
    client: Option<String>,
    hypervisor: Option<String>,
    hyperdrive: Option<String>
}

#[derive(Debug)]
struct RunningStatus {
    process_state: ProcessState,
    component_statuses: ComponentStatuses,
    network: GolemNet,
    golem_version: String,
    node_name: String,
    disk_usage: String
}

#[derive(Debug)]
struct NetworkStatus {
    is_connected: bool,
    port_status: Map<u16, String>,
    nodes_online: usize
}

#[derive(Debug)]
struct AccountStatus {
    eth_address: String,
    gnt_available: String,
    eth_available: String
}

#[derive(Debug)]
struct ProviderStatus {
    subtasks_accepted: u32,
    subtasks_rejected: u32,
    subtasks_failed: u32,
    subtasks_computed: u32,
    subtasks_in_network: u32,
    provider_state: Option<String>,
    pending_payments: BigDecimal
}

#[derive(Debug)]
struct RequestorStatus {
    is_any_task_active: bool,
    tasks_progress: String
}

#[derive(Debug)]
struct FormattedGeneralStatus {
    running_status: Option<RunningStatus>,
    net_status: Option<NetworkStatus>,
    account_status: Option<AccountStatus>,
    provider_status: Option<ProviderStatus>,
    requestor_status: Option<RequestorStatus>
}

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item=CommandResponse, Error=Error> + 'static> {
        match self {
            &Section::Status {} => Box::new(self.status(endpoint))
        }
    }

    pub fn status(&self, endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static)
                  -> impl Future<Item=CommandResponse, Error=Error> + 'static {

        /*let status = self.get_network_status(endpoint)
            .and_then(|network_status| {
            ok(FormattedGeneralStatus {
                running_status: None, //self.get_running_status(endpoint),
                net_status: Some(network_status),
                account_status: None, //self.get_account_status(endpoint),
                provider_status: None, //self.get_provider_status(endpoint),
                requestor_status: None, //self.get_requestor_status(endpoint)
            }).and_then(|general_status| {
                self.get_provider_status(endpoint).and_then(|provider_status|) {

                }
                general_status
            })
//            futures::future::ok(CommandResponse::FormattedObject(Box::new(x)))
        });
        status*/
        let status = self.get_network_status(endpoint.clone())
            .join3(self.get_provider_status(endpoint.clone()), self.get_running_status(endpoint.clone()))
            .map(|(net_status, provider_status, running_status)| {
                let x = FormattedGeneralStatus {
                    running_status: Some(running_status),
                    net_status: Some(net_status),
                    account_status: None, //Some(self.get_account_status(endpoint)),
                    provider_status: Some(provider_status), //self.get_provider_status(endpoint),
                    requestor_status: None, //self.get_requestor_status(endpoint)
                };
                CommandResponse::FormattedObject(Box::new(x))

            });
        status
    }

    fn get_running_status(&self, endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static) ->
        impl Future<Item = RunningStatus, Error = Error> {

        let is_mainnet = endpoint.as_golem().is_mainnet().from_err(); // <- to sa futuresy
        let server_status = endpoint.as_golem().status().from_err(); // <- to sa futuresy
        let node_info = endpoint.as_golem_net().get_node().from_err();
        let version = endpoint.as_golem().get_version().from_err();

        is_mainnet.join4(server_status, node_info, version).map(|(is_mainnet, server_status, node_info, version)|
            RunningStatus {
                process_state: ProcessState::Running,
                network: if is_mainnet { GolemNet::Mainnet } else { GolemNet::Testnet },
                component_statuses: ComponentStatuses {
                    docker_status: server_status.docker.map(|component_report|
                        String::from(map_statuses("docker", &component_report.0, &component_report.1))),
                    client: server_status.client.map(|component_report|
                        String::from(map_statuses("client", &component_report.0, &component_report.1))),
                    hyperdrive: server_status.hyperdrive.map(|component_report|
                        String::from(map_statuses("hyperdrive", &component_report.0, &component_report.1))),
                    hypervisor: server_status.hypervisor.map(|component_report|
                        String::from(map_statuses("hypervisor", &component_report.0, &component_report.1)))
                },
                disk_usage: String::from(""),
                golem_version: version,
                node_name: node_info.node_name
            }
        )
    }


    fn get_network_status(&self, endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static) ->
        impl Future<Item = NetworkStatus, Error = Error>
    {
        let net_status = endpoint.as_golem_net().connection_status().from_err();
        let online_nodes = endpoint.as_golem_net().get_connected_peers().from_err();

        net_status.join(online_nodes).map(|(net_status, online_nodes)|
            NetworkStatus {
                is_connected: net_status.connected,
                port_status: net_status.port_statuses,
                nodes_online: online_nodes.len()
            })
    }

    fn get_account_status(&self, endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static) ->
        impl Future<Item = AccountStatus, Error = Error> {

        let payment_address = endpoint.as_golem_pay().get_pay_ident().from_err();
        let balance = endpoint.as_golem_pay().get_pay_balance().from_err();

        payment_address.join(balance).map(|(payment_address, balance) |
            AccountStatus {
                eth_address: payment_address.clone(),
                gnt_available: crate::eth::Currency::GNT.format_decimal(&balance.av_gnt),
                eth_available: crate::eth::Currency::ETH.format_decimal(&balance.eth)
            })
    }

    fn get_provider_status(&self, endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static) ->
        impl Future<Item = ProviderStatus, Error = Error> {

        let task_stats = endpoint.as_golem_comp().get_tasks_stats().from_err();
        let awaiting_incomes = endpoint.as_golem_pay().get_incomes_list().from_err();

        task_stats.join(awaiting_incomes).map(|(task_stats, awaiting_incomes)|
            ProviderStatus {
                subtasks_accepted: task_stats.subtasks_accepted.session,
                subtasks_rejected: task_stats.subtasks_rejected.session,
                subtasks_failed: task_stats.subtasks_with_errors.session,
                subtasks_computed: task_stats.subtasks_computed.session,
                subtasks_in_network: task_stats.in_network,
                provider_state: task_stats.provider_state.get("status").cloned(),
                pending_payments: awaiting_incomes.iter()
                    .filter(|income| mem::discriminant(&income.status) == mem::discriminant(&PaymentStatus::Awaiting))
                    .map(|x| &x.value)
                    .fold(bigdecimal::BigDecimal::zero(), |sum, val| sum + val)
            })
    }
    fn get_requestor_status(&self, endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static) ->
         impl Future<Item = RequestorStatus, Error = Error> {
        futures::future::ok(
        RequestorStatus {
                is_any_task_active: false,
                tasks_progress: String::from("")
            }
        )
    }
}

impl FormattedGeneralStatus {
//    fn print_network_status(&self, out: &mut Box<io::Stdout>){
//        write!(*out, "{}:\n\tConnected: {}\n\tNumber of nodes in the network: {}\n",
//               Style::new().bold().underline().paint("Network"),
//               if self.net_status.connected {Green.paint("ONLINE") } else {Red.paint("OFFLINE")},
//               self.nodes_online);
//    }
//    fn print_tasks_status(&self, out: &mut Box<io::Stdout>){
//        write!(*out, "{}:\n\tSubtasks accepted (in session): {}\n\t\
//        Subtasks rejected (in session): {}\n\tSubtasks failed (in session): {}\n\tSubtasks computed (in session): {}\n\t\
//        Subtasks in network: {}\n", Style::new().bold().underline().paint("Computation"),
//        self.computation_status.subtasks_accepted, self.computation_status.subtasks_rejected,
//               self.computation_status.subtasks_failed, self.computation_status.subtasks_computed, self.computation_status.subtasks_in_network);
////        self.computation_status.provider_state.unwrap_or(String::from("Unknown"))
//    }

}

impl FormattedObject for FormattedGeneralStatus {
    fn to_json(&self) -> Result<Value, Error> {
        Ok(serde_json::from_str("{}").unwrap())
    }

    fn print(&self) -> Result<(), Error> {
        println!("{:?}", self);
        let mut stdout = Box::new(io::stdout()); // Why I box that?

        Ok(())
    }
}
