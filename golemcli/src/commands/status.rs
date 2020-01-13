use crate::component_response::map_statuses;
use crate::context::*;
use actix::prelude::*;
use ansi_term::Colour::{Green, Red};
use ansi_term::{Colour, Style};
use bigdecimal::{BigDecimal, Zero};
use failure::Fallible;
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
use prettytable::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::btree_map::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs::File;
use std::io::{self, StdoutLock, Write};
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
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

const UNDERLINE_TITLE_WIDTH: usize = 25;

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
    indentation_mark: &'static str,
    content: Vec<Vec<SectionEntry>>,
    current_section: Vec<SectionEntry>,
}

#[derive(Clone, Debug)]
enum SectionEntry {
    StartSection { title: String },
    StartSubSection { title: String },
    EndSection,
    EndSubSection,
    Entry { key: String, value: String },
}

impl SectionBuilder {
    fn new(indentation_mark: &'static str) -> SectionBuilder {
        SectionBuilder {
            indentation_mark,
            content: Vec::new(),
            current_section: Vec::new(),
        }
    }
    fn new_section(&mut self, title: impl Into<String>) -> &mut Self {
        let title = title.into();
        self.current_section
            .push(SectionEntry::StartSection { title });
        self
    }

    fn new_subsection(&mut self, title: impl Into<String>) -> &mut Self {
        let title = title.into();
        self.current_section
            .push(SectionEntry::StartSubSection { title });
        self
    }

    fn end_section(&mut self) -> &mut Self {
        self.current_section.push(SectionEntry::EndSection);
        self.content.push(self.current_section.clone());
        self.current_section = Vec::new();
        self
    }

    fn end_subsection(&mut self) -> &mut Self {
        self.current_section.push(SectionEntry::EndSubSection);
        self
    }

    fn entry(&mut self, key: &str, value: &str) -> &mut Self {
        self.current_section.push(SectionEntry::Entry {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    fn fill(&self, len: usize) -> String {
        " ".repeat(UNDERLINE_TITLE_WIDTH - len)
    }

    fn make_ident(&self, ident: u8) -> String {
        self.indentation_mark.repeat(ident.into())
    }

    fn section_to_table(&self, section: &Vec<SectionEntry>) -> Table {
        let mut table = Table::new();
        let format = format::FormatBuilder::new().padding(1, 1).build();
        table.set_format(format);
        let mut ident = 0;
        for item in section {
            match item {
                SectionEntry::StartSection { title } => {
                    table.add_row(row![Style::new()
                        .fg(Colour::Yellow)
                        .underline()
                        .paint(format!("{}{}", title, self.fill(title.len())))]);
                }
                SectionEntry::EndSection => {
                    table.add_empty_row();
                }
                SectionEntry::Entry { key, value } => {
                    table.add_row(row![
                        format!(
                            "{}{}",
                            self.make_ident(ident),
                            Style::new().bold().paint(format!("{}:", key))
                        ),
                        value
                    ]);
                }
                SectionEntry::EndSubSection => {
                    assert!(ident > 0);
                    ident = ident - 1;
                }
                SectionEntry::StartSubSection { title } => {
                    table.add_row(row![
                        format!(
                            "{}{}\n",
                            self.make_ident(ident),
                            Style::new().bold().paint(format!("{}:", title))
                        ),
                        ""
                    ]);
                    ident = ident + 1;
                }
            };
        }
        table
    }

    fn to_table(&self) -> prettytable::Table {
        use prettytable::*;

        let mut table = Table::new();
        let mut left_column = Table::new();
        let mut right_column = Table::new();
        let format = format::FormatBuilder::new().padding(2, 0).build();
        table.set_format(format);
        left_column.set_format(format);
        right_column.set_format(format);

        let mut left_sections = self.content.clone();
        let right_sections = left_sections.split_off(self.content.len() / 2);

        for sec in left_sections {
            left_column.add_row(row![self.section_to_table(&sec)]);
        }
        for sec in right_sections {
            right_column.add_row(row![self.section_to_table(&sec)]);
        }
        table.add_row(row![left_column, right_column]);
        table
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
    provider_state: String,
    pending_payments: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FormattedGeneralStatus {
    running_status: RunningStatus,
    net_status: NetworkStatus,
    account_status: AccountStatus,
    provider_status: ProviderStatus,
    requestor_tasks_progress: String,
}

impl Section {
    pub async fn run(
        &self,
        ctx: &CliCtx,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> failure::Fallible<CommandResponse> {
        self.status(endpoint, ctx).await
    }

    async fn status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        ctx: &CliCtx,
    ) -> failure::Fallible<CommandResponse> {
        let (net_status, provider_status, running_status, requestor_tasks_progress, account_status) =
            future::try_join5(
                self.get_network_status(endpoint.clone()),
                self.get_provider_status(endpoint.clone()),
                self.get_running_status(endpoint.clone(), ctx),
                self.get_requestor_status(endpoint.clone()),
                self.get_account_status(endpoint.clone()),
            )
            .await?;

        Ok(CommandResponse::FormattedObject(Box::new(
            FormattedGeneralStatus {
                running_status,
                net_status,
                account_status,
                provider_status,
                requestor_tasks_progress,
            },
        )))
    }

    fn check_is_golem_run(is_mainnet: bool, ctx: &CliCtx) -> bool {
        let lock_path = ctx.get_golem_lock_path(is_mainnet);

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
            Err(_) => true,
        }
    }

    async fn get_running_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        _ctx: &CliCtx,
    ) -> Fallible<RunningStatus> {
        let (is_mainnet, server_status, node_info, version, disk_usage) = future::try_join5(
            endpoint.as_golem().is_mainnet(),
            endpoint.as_golem().status(),
            endpoint.as_golem_net().get_node(),
            endpoint.as_golem().get_version(),
            endpoint.as_golem_res().get_res_dirs_sizes(),
        )
        .await?;

        let is_golem_running = true; //Section::check_is_golem_run(is_mainnet, ctx);
        Ok(RunningStatus {
            process_state: match is_golem_running {
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
        })
    }

    async fn get_network_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<NetworkStatus> {
        let (net_status, online_nodes) = future::try_join(
            endpoint.as_golem_net().connection_status(),
            endpoint.as_golem_net().get_connected_peers(),
        )
        .await?;
        Ok(NetworkStatus {
            is_connected: net_status.connected,
            port_status: net_status.port_statuses,
            nodes_online: online_nodes.len(),
        })
    }

    async fn get_account_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<AccountStatus> {
        let (payment_address, balance) = future::try_join(
            endpoint.as_golem_pay().get_pay_ident(),
            endpoint.as_golem_pay().get_pay_balance(),
        )
        .await?;
        Ok(AccountStatus {
            eth_address: payment_address.clone(),
            gnt_available: crate::eth::Currency::GNT.format_decimal(&balance.av_gnt.with_prec(3)),
            eth_available: crate::eth::Currency::ETH.format_decimal(&balance.eth),
        })
    }

    async fn get_provider_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<ProviderStatus> {
        let (task_stats, awaiting_incomes) = future::try_join(
            endpoint.as_golem_comp().get_tasks_stats(),
            endpoint.as_golem_pay().get_incomes_list(),
        )
        .await?;
        Ok(ProviderStatus {
            subtasks_accepted: task_stats.subtasks_accepted.session,
            subtasks_rejected: task_stats.subtasks_rejected.session,
            subtasks_failed: task_stats.subtasks_with_errors.session,
            subtasks_computed: task_stats.subtasks_computed.session,
            subtasks_in_network: task_stats.in_network,
            provider_state: task_stats.provider_state.status,
            pending_payments: crate::eth::Currency::GNT.format_decimal(
                &awaiting_incomes
                    .iter()
                    .filter(|income| {
                        mem::discriminant(&income.status)
                            == mem::discriminant(&PaymentStatus::Awaiting)
                    })
                    .map(|x| &x.value)
                    .fold(bigdecimal::BigDecimal::zero(), |sum, val| sum + val),
            ),
        })
    }

    async fn get_requestor_status(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> failure::Fallible<String> {
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

        let tasks: Vec<TaskInfo> = endpoint.as_golem_comp().get_tasks().await?;
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

        let all_subtasks = future::try_join_all(subtasks).await?;
        let finished_subtasks = all_subtasks
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
            });

        Ok(if total_subtasks > 0 {
            format!("{}/{}", finished_subtasks, total_subtasks)
        } else {
            "0/0".into()
        })
    }
}

impl FormattedObject for FormattedGeneralStatus {
    fn to_json(&self) -> Result<Value, Error> {
        Ok(serde_json::to_value(&self)?)
    }

    fn print(&self) -> Result<(), Error> {
        let mut section_builder = SectionBuilder::new("  ");

        section_builder
            .new_section("General")
            .entry(
                &String::from("Process State"),
                &self.running_status.process_state.to_string(),
            )
            .new_subsection(String::from("Components Status"))
            .entry(
                "docker",
                self.running_status
                    .component_statuses
                    .docker_status
                    .as_ref()
                    .map(AsRef::as_ref)
                    .unwrap_or("unknown"),
            )
            .entry(
                "hyperdrive",
                self.running_status
                    .component_statuses
                    .hyperdrive
                    .as_ref()
                    .map(AsRef::as_ref)
                    .unwrap_or("unknown"),
            )
            .entry(
                "client",
                self.running_status
                    .component_statuses
                    .client
                    .as_ref()
                    .map(AsRef::as_ref)
                    .unwrap_or("unknown"),
            );

        if self.running_status.component_statuses.hypervisor.is_some() {
            section_builder.entry(
                "hypervisor",
                self.running_status
                    .component_statuses
                    .hypervisor
                    .as_ref()
                    .unwrap(),
            );
        }
        let connection_status = if self.net_status.is_connected {
            "ONLINE"
        } else {
            "OFFLINE"
        };

        section_builder
            .end_subsection()
            .entry(
                &String::from("Golem version"),
                &self.running_status.golem_version,
            )
            .entry(&String::from("Node name"), &self.running_status.node_name)
            .entry("Network", &self.running_status.network.to_string())
            .entry("Disk usage", &self.running_status.disk_usage.received_files)
            .end_section()
            .new_section("Network")
            .entry("Connection", connection_status)
            .new_subsection("Port statuses");

        for (key, value) in self.net_status.port_status.iter() {
            section_builder.entry(&key.to_string(), value);
        }

        section_builder
            .end_subsection()
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
                &String::from("Subtasks rejected (in session)"),
                &self.provider_status.subtasks_rejected.to_string(),
            )
            .entry(
                &String::from("Subtasks failed (in session)"),
                &self.provider_status.subtasks_failed.to_string(),
            )
            .entry(
                &String::from("Subtasks computed (in session)"),
                &self.provider_status.subtasks_computed.to_string(),
            )
            .entry(
                &String::from("Tasks in network (in session)"),
                &self.provider_status.subtasks_in_network.to_string(),
            )
            .entry(
                &String::from("Pending payments"),
                &self.provider_status.pending_payments,
            );
        section_builder.entry(
            "Provider state",
            &self.provider_status.provider_state.as_ref(),
        );

        section_builder.end_section();

        section_builder.new_section("Requestor status");
        section_builder.entry(
            "all computed subtasks / all subtasks",
            self.requestor_tasks_progress.as_ref(),
        );
        section_builder.end_section();
        section_builder.to_table().printstd();
        Ok(())
    }
}
