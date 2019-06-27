use crate::component_response::map_statuses;
use crate::context::*;
use actix::prelude::*;
use ansi_term::Colour::{Green, Red};
use ansi_term::Style;
use bigdecimal::{BigDecimal, Zero};
use fs2::FileExt;
use futures::future::{ok, Future, Join};
use futures::prelude::*;
use golem_rpc_api::comp::{
    AsGolemComp, SubtaskInfo, SubtaskStats, SubtaskStatus, TaskInfo, TaskStatus,
};
use golem_rpc_api::core::ServerStatus;
use golem_rpc_api::net::{AsGolemNet, NetStatus, NodeInfo, PeerInfo};
use golem_rpc_api::pay::{Balance, PaymentStatus};
use golem_rpc_api::res::CacheSizes;
use golem_rpc_api::rpc::AsInvoker;
use golem_rpc_api::settings::DynamicSetting;
use golem_rpc_api::{core::AsGolemCore, pay::AsGolemPay, res::AsGolemRes, settings, Map};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_derive::*;
use serde_json::Value;
use std::collections::btree_map::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs::File;
use std::io::{self, StdoutLock, Write};
use std::net::ToSocketAddrs;
use std::path::Path;
use std::sync::atomic::Ordering::SeqCst;
use std::{fmt, mem};
use structopt::{clap, StructOpt};

#[derive(StructOpt, Debug)]
pub struct Section {}

#[derive(Debug, Serialize, Deserialize)]
enum ProcessState {
    Running,
    Stopped,
}

#[derive(Debug, Serialize, Deserialize)]
enum GolemNet {
    Mainnet,
    Testnet,
}

impl Display for GolemNet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GolemNet::Mainnet => write!(f, "mainnet"),
            GolemNet::Testnet => write!(f, "testnet"),
        }
    }
}

impl Display for ProcessState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProcessState::Running => write!(f, "running"),
            ProcessState::Stopped => write!(f, "stopped"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ComponentStatuses {
    docker_status: Option<String>,
    client: Option<String>,
    hypervisor: Option<String>,
    hyperdrive: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RunningStatus {
    process_state: ProcessState,
    component_statuses: ComponentStatuses,
    network: GolemNet,
    golem_version: String,
    node_name: String,
    disk_usage: CacheSizes,
}

struct SectionBuilder {
    indentation_size: u8,
    indentation_mark: &'static str,
    content: Vec<SectionEntry>,
}

enum SectionEntry {
    StartSection { title: &'static str },
    StartSubSection { title: &'static str },
    EndSection,
    Entry { key: &'static str, value: String },
}

impl SectionBuilder {
    fn new(indentation_size: u8, indentation_mark: &'static str) -> SectionBuilder {
        SectionBuilder {
            indentation_size,
            indentation_mark,
            content: Vec::new(),
        }
    }
    fn new_section(&mut self, title: &'static str) -> &mut Self {
        self.content.push(SectionEntry::StartSection { title });
        self
    }

    fn new_subsection(&mut self, title: &'static str) -> &mut Self {
        self.content.push(SectionEntry::StartSubSection { title });
        self
    }

    fn end_section(&mut self) -> &mut Self {
        self.content.push(SectionEntry::EndSection);
        self
    }

    fn entry(&mut self, key: &'static str, value: &String) -> &mut Self {
        self.content.push(SectionEntry::Entry {
            key,
            value: value.clone(),
        });
        self
    }

    fn make_ident(&self, ident: u8) -> String {
        self.indentation_mark.repeat(ident.into())
    }

    fn build(&mut self) -> String {
        let mut ident: u8 = 0;
        let mut result = String::new();

        for item in &self.content {
            match item {
                SectionEntry::StartSection { title } => {
                    result.push_str(
                        format!(
                            "{}{}\n",
                            self.make_ident(ident),
                            Style::new().underline().bold().paint(format!("{}", title))
                        )
                        .as_ref(),
                    );
                    ident = ident + 1;
                }
                SectionEntry::EndSection => {
                    assert!(ident > 0);
                    ident = ident - 1;
                }
                SectionEntry::Entry { key, value } => result.push_str(
                    format!(
                        "{}{} {}\n",
                        self.make_ident(ident),
                        Style::new().bold().paint(format!("{}:", key)),
                        value
                    )
                    .as_ref(),
                ),
                SectionEntry::StartSubSection { title } => {
                    result.push_str(
                        format!(
                            "{}{}\n",
                            self.make_ident(ident),
                            Style::new().bold().paint(format!("{}:", title))
                        )
                        .as_ref(),
                    );
                    ident = ident + 1;
                }
            }
        }
        result
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NetworkStatus {
    is_connected: bool,
    port_status: Map<u16, String>,
    nodes_online: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccountStatus {
    eth_address: String,
    gnt_available: String,
    eth_available: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProviderStatus {
    subtasks_accepted: u32,
    subtasks_rejected: u32,
    subtasks_failed: u32,
    subtasks_computed: u32,
    subtasks_in_network: u32,
    provider_state: Option<String>,
    pending_payments: BigDecimal,
}

#[derive(Debug, Serialize, Deserialize)]
struct FormattedGeneralStatus {
    running_status: RunningStatus,
    net_status: NetworkStatus,
    account_status: AccountStatus,
    provider_status: ProviderStatus,
    requestor_tasks_progress: Option<String>,
}

impl Section {
    pub fn run(
        &self,
        ctx: &CliCtx,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        Box::new(self.status(endpoint, ctx))
    }

    pub fn status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        ctx: &CliCtx,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let is_golem_run = self.check_is_golem_run(ctx);

        let status = self
            .get_network_status(endpoint.clone())
            .join5(
                self.get_provider_status(endpoint.clone()),
                self.get_running_status(endpoint.clone(), is_golem_run),
                self.get_requestor_status(endpoint.clone()),
                self.get_account_status(endpoint.clone()),
            )
            .map(
                |(
                    net_status,
                    provider_status,
                    running_status,
                    requestor_tasks_progress,
                    account_status,
                )| {
                    let x = FormattedGeneralStatus {
                        running_status,
                        net_status,
                        account_status,
                        provider_status,
                        requestor_tasks_progress,
                    };
                    CommandResponse::FormattedObject(Box::new(x))
                },
            );
        status
    }

    fn check_is_golem_run(&self, ctx: &CliCtx) -> bool {
        let lock_path = ctx.get_golem_lock_path();
        let lock_file_path = lock_path.to_str();

        if lock_file_path.is_none() {
            return true;
        }
        let file = File::open(lock_file_path.unwrap());

        if file.is_err() {
            return false;
        }
        let is_golem_running = file.unwrap().try_lock_exclusive();
        match is_golem_running {
            Ok(_) => false,
            Err(e) => true,
        }
    }

    fn get_running_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        is_golem_run: bool,
    ) -> impl Future<Item = RunningStatus, Error = Error> {
        let is_mainnet = endpoint.as_golem().is_mainnet().from_err();
        let server_status = endpoint.as_golem().status().from_err();
        let node_info = endpoint.as_golem_net().get_node().from_err();
        let version = endpoint.as_golem().get_version().from_err();
        let disk_usage = endpoint.as_golem_res().get_res_dirs_sizes().from_err();

        is_mainnet
            .join5(server_status, node_info, version, disk_usage)
            .map(
                move |(is_mainnet, server_status, node_info, version, disk_usage)| {
                    println!("status = {:?}", server_status);
                    RunningStatus {
                        process_state: match is_golem_run {
                            true => ProcessState::Running,
                            false => ProcessState::Stopped,
                        },
                        network: if is_mainnet {
                            GolemNet::Mainnet
                        } else {
                            GolemNet::Testnet
                        },
                        component_statuses: ComponentStatuses {
                            docker_status: server_status.docker.map(|component_report| {
                                String::from(map_statuses(
                                    "docker",
                                    &component_report.0,
                                    &component_report.1,
                                ))
                            }),
                            client: server_status.client.map(|component_report| {
                                String::from(map_statuses(
                                    "client",
                                    &component_report.0,
                                    &component_report.1,
                                ))
                            }),
                            hyperdrive: server_status.hyperdrive.map(|component_report| {
                                String::from(map_statuses(
                                    "hyperdrive",
                                    &component_report.0,
                                    &component_report.1,
                                ))
                            }),
                            hypervisor: server_status.hypervisor.map(|component_report| {
                                String::from(map_statuses(
                                    "hypervisor",
                                    &component_report.0,
                                    &component_report.1,
                                ))
                            }),
                        },
                        disk_usage: disk_usage,
                        golem_version: version,
                        node_name: node_info.node_name,
                    }
                },
            )
    }

    fn get_network_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = NetworkStatus, Error = Error> {
        let net_status = endpoint.as_golem_net().connection_status().from_err();
        let online_nodes = endpoint.as_golem_net().get_connected_peers().from_err();

        net_status
            .join(online_nodes)
            .map(|(net_status, online_nodes)| NetworkStatus {
                is_connected: net_status.connected,
                port_status: net_status.port_statuses,
                nodes_online: online_nodes.len(),
            })
    }

    fn get_account_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = AccountStatus, Error = Error> {
        let payment_address = endpoint.as_golem_pay().get_pay_ident().from_err();
        let balance = endpoint.as_golem_pay().get_pay_balance().from_err();

        payment_address
            .join(balance)
            .map(|(payment_address, balance)| AccountStatus {
                eth_address: payment_address.clone(),
                gnt_available: crate::eth::Currency::GNT.format_decimal(&balance.av_gnt),
                eth_available: crate::eth::Currency::ETH.format_decimal(&balance.eth),
            })
    }

    fn get_provider_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = ProviderStatus, Error = Error> {
        let task_stats = endpoint.as_golem_comp().get_tasks_stats().from_err();
        let awaiting_incomes = endpoint.as_golem_pay().get_incomes_list().from_err();

        task_stats
            .join(awaiting_incomes)
            .map(|(task_stats, awaiting_incomes)| ProviderStatus {
                subtasks_accepted: task_stats.subtasks_accepted.session,
                subtasks_rejected: task_stats.subtasks_rejected.session,
                subtasks_failed: task_stats.subtasks_with_errors.session,
                subtasks_computed: task_stats.subtasks_computed.session,
                subtasks_in_network: task_stats.in_network,
                provider_state: task_stats.provider_state.get("status").cloned(),
                pending_payments: awaiting_incomes
                    .iter()
                    .filter(|income| {
                        mem::discriminant(&income.status)
                            == mem::discriminant(&PaymentStatus::Awaiting)
                    })
                    .map(|x| &x.value)
                    .fold(bigdecimal::BigDecimal::zero(), |sum, val| sum + val),
            })
    }

    fn get_requestor_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = Option<String>, Error = Error> {
        let active_tasks = vec![
            TaskStatus::Restarted,
            TaskStatus::Computing,
            TaskStatus::CreatingDeposit,
            TaskStatus::Sending,
            TaskStatus::Waiting,
            TaskStatus::Starting,
        ];

        let mut task_status_in_fly = active_tasks.clone();

        task_status_in_fly.extend(vec![TaskStatus::NotStarted]);

        endpoint
            .as_golem_comp()
            .get_tasks()
            .and_then(move |tasks: Vec<TaskInfo>| {
                let mut subtasks = Vec::new();

                let tasks_in_fly: Vec<TaskInfo> = tasks
                    .into_iter()
                    .filter(|task| {
                        task_status_in_fly.contains(&task.status) && task.subtasks_count.is_some()
                    })
                    .collect();
                let total_subtasks = tasks_in_fly.iter().fold(0, |total_subtasks, task| {
                    total_subtasks + task.subtasks_count.unwrap()
                });
                subtasks.reserve(tasks_in_fly.len() * 2);
                for task_in_fly in &tasks_in_fly {
                    let subtask = endpoint
                        .as_golem_comp()
                        .get_subtasks(task_in_fly.id.clone());
                    subtasks.push(subtask);
                }
                futures::future::join_all(subtasks).join(futures::future::ok(total_subtasks))
            })
            .map(|(all_subtasks, total_subtasks)| {
                (
                    total_subtasks,
                    all_subtasks
                        .into_iter()
                        .fold(0, |finished_subtasks, subtasks| {
                            finished_subtasks
                                + subtasks.map_or_else(
                                    || 0,
                                    |subtasks: Vec<SubtaskInfo>| {
                                        subtasks
                                            .iter()
                                            .filter(|subtask| {
                                                mem::discriminant(&subtask.status)
                                                    == mem::discriminant(&SubtaskStatus::Finished)
                                            })
                                            .count()
                                    },
                                )
                        }),
                )
            })
            .map(
                |(total_subtasks, finished_subtasks)| match total_subtasks > 0 {
                    true => Some(format!("{}/{}", finished_subtasks, total_subtasks)),
                    false => None,
                },
            )
            .from_err()
    }
}

impl FormattedObject for FormattedGeneralStatus {
    fn to_json(&self) -> Result<Value, Error> {
        Ok(serde_json::to_value(&self)?)
    }

    fn print(&self) -> Result<(), Error> {
        let mut section_builder = SectionBuilder::new(2, "  ");

        section_builder
            .new_section("General")
            .entry(
                "Process State",
                &self.running_status.process_state.to_string(),
            )
            .new_subsection("Components Status")
            .entry(
                "docker",
                self.running_status
                    .component_statuses
                    .docker_status
                    .as_ref()
                    .unwrap_or(&String::from("unknown")),
            )
            .entry(
                "hyperdrive",
                self.running_status
                    .component_statuses
                    .hyperdrive
                    .as_ref()
                    .unwrap_or(&String::from("unknown")),
            )
            .entry(
                "client",
                self.running_status
                    .component_statuses
                    .client
                    .as_ref()
                    .unwrap_or(&String::from("unknown")),
            );

        if self.running_status.component_statuses.hypervisor.is_some() {
            section_builder.entry(
                "hypervisor",
                self.running_status
                    .component_statuses
                    .client
                    .as_ref()
                    .unwrap(),
            );
        }
        let connection_status = if self.net_status.is_connected {
            String::from("ONLINE")
        } else {
            String::from("OFFLINE")
        };

        section_builder
            .end_section()
            .entry("Golem version", &self.running_status.golem_version)
            .entry("Node name", &self.running_status.node_name)
            .entry("Network", &self.running_status.network.to_string())
            .new_subsection("Disk usage")
            .entry(
                "Received files",
                &self.running_status.disk_usage.received_files,
            )
            .entry(
                "Distributed files",
                &self.running_status.disk_usage.distributed_files,
            )
            .end_section()
            .end_section()
            .new_section("Network")
            .entry("Connection", &connection_status)
            .entry(
                "Port statuses",
                &format!("{:?}", self.net_status.port_status),
            )
            .entry("Nodes online", &self.net_status.nodes_online.to_string())
            .end_section()
            .new_section("Account")
            .entry("ETH address", &self.account_status.eth_address)
            .entry("GNT available", &self.account_status.gnt_available)
            .entry("ETH available", &self.account_status.eth_available)
            .end_section()
            .new_section("Provider Status")
            .entry(
                "Subtasks accepted (in session)",
                &self.provider_status.subtasks_accepted.to_string(),
            )
            .entry(
                "Subtasks rejected (in session)",
                &self.provider_status.subtasks_rejected.to_string(),
            )
            .entry(
                "Subtasks failed (in session)",
                &self.provider_status.subtasks_failed.to_string(),
            )
            .entry(
                "Subtasks computed (in session)",
                &self.provider_status.subtasks_computed.to_string(),
            )
            .entry(
                "Tasks in network (in session)",
                &self.provider_status.subtasks_in_network.to_string(),
            )
            .entry(
                "Pending payments",
                &self.provider_status.pending_payments.to_string(),
            );

        if self.provider_status.provider_state.is_some() {
            section_builder.entry(
                "Provider state",
                &self.provider_status.provider_state.as_ref().unwrap(),
            );
        };

        section_builder.end_section();

        let requestor_perpective = String::from("No active tasks");
        let requestor_status = self
            .requestor_tasks_progress
            .as_ref()
            .unwrap_or(&requestor_perpective);
        section_builder.new_section("Requestor status");
        section_builder.entry("Status", &requestor_status);
        section_builder.end_section();

        println!("{}", section_builder.build());
        Ok(())
    }
}
