use crate::context::*;
use actix::prelude::*;
use futures::prelude::*;
use golem_rpc_api::settings::DynamicSetting;
use golem_rpc_api::{core::AsGolemCore, settings, Map, pay::AsGolemPay, res::AsGolemRes};
use serde_json::Value;
use std::collections::btree_map::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::io::{self, Write, StdoutLock};
use futures::future::{Join, Future, ok};
use structopt::{clap, StructOpt};
use golem_rpc_api::comp::{AsGolemComp, SubtaskStats, TaskStatus, SubtaskInfo, TaskInfo, SubtaskStatus};
use golem_rpc_api::net::{AsGolemNet, NetStatus, PeerInfo, NodeInfo};
use golem_rpc_api::core::ServerStatus;
use ansi_term::Colour::{Red, Green};
use ansi_term::Style;
use crate::component_response::map_statuses;
use golem_rpc_api::rpc::AsInvoker;
use golem_rpc_api::pay::{PaymentStatus, Balance};
use bigdecimal::{Zero, BigDecimal};
use std::{mem, fmt};
use golem_rpc_api::res::CacheSizes;
use std::net::ToSocketAddrs;
use std::fmt::Display;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Change settings (unimplemented) TODO:
    #[structopt(name = "status")]
    Status {

    }
}

#[derive(Debug)]
enum ProcessState {
    Running,
    Stopped,
    Stopping
}

#[derive(Debug)]
enum GolemNet {
    Mainnet,
    Testnet
}

impl Display for GolemNet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GolemNet::Mainnet => write!(f, "mainnet"),
            GolemNet::Testnet => write!(f, "testnet")
        }
    }
}

impl Display for ProcessState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProcessState::Running => write!(f, "running"),
            ProcessState::Stopped => write!(f, "stopped"),
            ProcessState::Stopping => write!(f, "stopping"),
        }
    }
}

#[derive(Debug)]
struct ComponentStatuses {
    docker_status: Option<String>,
    client: Option<String>,
    hypervisor: Option<String>,
    hyperdrive: Option<String>
}

trait AsSection {
    fn to_section(&self, indentation: usize) -> String;
}

impl AsSection for ComponentStatuses {
    fn to_section(&self, indentation: usize) -> String {
        format!("{}docker status: {}\n{}", "\t".repeat(indentation), self.docker_status.as_ref().unwrap_or(&String::from("unknown")),
        format!("{}client: {}\n{}", "\t".repeat(indentation), self.client.as_ref().unwrap_or(&String::from("unknown")),
        format!("{}hyperdrive status: {}\n{}", "\t".repeat(indentation), self.hyperdrive.as_ref().unwrap_or(&String::from("unknown")),
        self.hypervisor.as_ref().map_or("".to_string(), |hv_status| format!("{}hypervisor: {}\n", "\t".repeat(indentation), hv_status)))))
    }
}

#[derive(Debug)]
struct RunningStatus {
    process_state: ProcessState,
    component_statuses: ComponentStatuses,
    network: GolemNet,
    golem_version: String,
    node_name: Option<String>,
    disk_usage: CacheSizes
}

impl AsSection for RunningStatus {
    fn to_section(&self, indentation: usize) -> String {
        format!("{}Process state: {}\n{}", "\t".repeat(indentation), &self.process_state,
                format!("{}Component statuses: {}\n{}", "\t".repeat(indentation), self.component_statuses.to_section(indentation + 1),
                        format!("{}Network: {}\n{}Golem version: {}\n{}", "\t".repeat(indentation), "\t".repeat(indentation), self.network, self.golem_version,
                                self.node_name.as_ref().map_or("".to_string(), |hv_status| format!("{}Node name: {}\n", "\t".repeat(indentation), hv_status)
                                )
                        )
                )
        )
    }
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
    tasks_progress: Option<String>
}

#[derive(Debug)]
struct FormattedGeneralStatus {
    running_status: RunningStatus,
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

        let status = self.get_network_status(endpoint.clone())
            .join5(self.get_provider_status(endpoint.clone()), self.get_running_status(endpoint.clone()), self.get_requestor_status(endpoint.clone()),
                   self.get_account_status(endpoint.clone()))
            .map(|(net_status, provider_status, running_status, requestor_status, account_status)| {
                let x = FormattedGeneralStatus {
                    running_status: running_status,
                    net_status: Some(net_status),
                    account_status: Some(account_status),
                    provider_status: Some(provider_status),
                    requestor_status: Some(requestor_status),
                };
                CommandResponse::FormattedObject(Box::new(x))

            });
        status
    }

    fn get_running_status(&self, endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static) ->
        impl Future<Item = RunningStatus, Error = Error> {

        let is_mainnet = endpoint.as_golem().is_mainnet().from_err();
        let server_status = endpoint.as_golem().status().from_err();
        let node_info = endpoint.as_golem_net().get_node().from_err();
        let version = endpoint.as_golem().get_version().from_err();
        let disk_usage = endpoint.as_golem_res().get_res_dirs_sizes().from_err();

        is_mainnet.join5(server_status, node_info, version, disk_usage).map(|(is_mainnet, server_status, node_info, version, disk_usage)|
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
                disk_usage: disk_usage,
                golem_version: version,
                node_name: if node_info.node_name.len() > 0 { Some(node_info.node_name)} else {None}
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

        let active_tasks = vec![TaskStatus::Restarted, TaskStatus::Computing,
                                TaskStatus::CreatingDeposit, TaskStatus::Sending,
                                TaskStatus::Waiting, TaskStatus::Starting];

        let mut task_status_in_fly = active_tasks.clone();

        task_status_in_fly.extend(vec![TaskStatus::NotStarted]);

        endpoint.as_golem_comp().get_tasks().and_then(
            move |tasks: Vec<TaskInfo>| {
                let mut subtasks = Vec::new();

                let tasks_in_fly : Vec<TaskInfo> = tasks.into_iter()
                    .filter(|task|
                        task_status_in_fly.contains(&task.status) && task.subtasks_count.is_some())
                    .collect();
                let total_subtasks = tasks_in_fly.iter().fold(0, |total_subtasks, task| total_subtasks + task.subtasks_count.unwrap());
                subtasks.reserve(tasks_in_fly.len() * 2);
                for task_in_fly in &tasks_in_fly {
                    let subtask = endpoint.as_golem_comp().get_subtasks(task_in_fly.id.clone());
                    subtasks.push(subtask);
                }
                //futures::future::ok(subtasks)
                 futures::future::join_all(subtasks).join(futures::future::ok(total_subtasks))
            }).map(|(all_subtasks, total_subtasks)|
                                              (total_subtasks, all_subtasks.into_iter().fold(0, |finished_subtasks, subtasks|
                    finished_subtasks +
                        subtasks.map_or_else(|| 0, |subtasks: Vec<SubtaskInfo>| subtasks.iter().filter(|subtask|
                            mem::discriminant(&subtask.status) == mem::discriminant(&SubtaskStatus::Finished)).count()))))
            .map(|(total_subtasks, finished_subtasks)|
                RequestorStatus {
                    tasks_progress:  if total_subtasks > 0 { Some(format!("{}/{}", finished_subtasks, total_subtasks)) } else {None}
                }
            ).from_err()
    }
}

impl FormattedObject for FormattedGeneralStatus {
    fn to_json(&self) -> Result<Value, Error> {
        Ok(serde_json::from_str("{}").unwrap())
//        Ok(serde_json::to_value(&self)?)
    }


    fn print(&self) -> Result<(), Error> {
        println!("{}\n{}",  Style::new().bold().paint("General:"), self.running_status.to_section(1));
        println!("------");
        println!("{:?}", self);
        let mut stdout = Box::new(io::stdout()); // Why I box that?

        Ok(())
    }
}
