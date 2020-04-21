use crate::context::*;
use crate::utils::GolemIdPattern;
use failure::Fallible;
use futures::{future, prelude::*};
use golem_rpc_api::core::AsGolemCore;
use golem_rpc_api::net::{
    ACLResult, AclRule, AclRuleItem, AclStatus, AsGolemNet, NodeInfo, PeerInfo,
};
use std::borrow::Borrow;
use std::collections::BTreeSet;
use std::net::IpAddr;
use structopt::{
    clap::{AppSettings, ArgSettings},
    StructOpt,
};

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Show current access list status
    #[structopt(name = "list")]
    List {
        ///
        #[structopt(long)]
        #[structopt(raw(hidden = "true"))]
        ip: bool,
        #[structopt(short = "l", long)]
        full: bool,
    },

    /// Creates new acl list with given configuration.
    #[structopt(name = "setup")]
    Setup(Setup),

    /// Allows interaction with given nodes.
    /// Removes from blacklist or adds to whitelist.
    #[structopt(name = "allow")]
    #[structopt(raw(setting = "AppSettings::ArgRequiredElseHelp"))]
    Allow {
        /// IPv4/IPv6 address, will be added to ip deny list.
        #[structopt(long)]
        #[structopt(raw(hidden = "true"))]
        ip: Vec<IpAddr>,

        /// GOLEM node id. it can be pattern in form <prefix>...<suffix>
        #[structopt(required = true)]
        node: Vec<GolemIdPattern>,
    },
    /// Deny interaction with given nodes.
    /// Adds for blacklist or removes from whitelist.
    #[structopt(name = "deny")]
    #[structopt(raw(setting = "AppSettings::ArgRequiredElseHelp"))]
    Deny {
        /// IPv4/IPv6 address, will be added to ip deny list.
        #[structopt(long)]
        #[structopt(raw(hidden = "true"))]
        ip: Vec<IpAddr>,

        /// GOLEM node id. it can be pattern in form <prefix>...<suffix>
        #[structopt(required = true)]
        node: Vec<GolemIdPattern>,

        /// Sets temporaty rule for given number of seconds.
        #[structopt(short = "s", long = "for-secs")]
        for_secs: Option<u32>,
    },
}

#[derive(StructOpt, Debug)]
pub enum Setup {
    /// Reset ACL to all nodes are allowed except listed.
    #[structopt(name = "all-except")]
    #[structopt(raw(display_order = "500"))]
    AllExcept {
        /// Banned GOLEM ids. Full id or pattern from list of known
        /// hosts
        nodes: Vec<GolemIdPattern>,
    },
    /// Reset ACL to only listed nodes are allowed.
    #[structopt(name = "only-listed")]
    #[structopt(raw(display_order = "500"))]
    OnlyListed {
        /// Initial list of GOLEM nodes that will be allowed to interact.
        #[structopt(required = true)]
        nodes: Vec<GolemIdPattern>,
    },
}

impl Section {
    pub async fn run(
        &self,
        ctx: &mut CliCtx,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        match self {
            Section::List { full, ip, .. } => list(endpoint, *full, *ip).await,
            Section::Deny { ip, node, for_secs } => {
                self.deny(endpoint, ip, node, for_secs.map(|s| s as i32).unwrap_or(-1))
                    .await
            }
            Section::Allow { ip, node } => self.allow(endpoint, ip, node).await,
            Section::Setup(Setup::AllExcept { nodes }) => {
                self.setup(endpoint, AclRule::Allow, nodes, ctx).await
            }
            Section::Setup(Setup::OnlyListed { nodes }) => {
                self.setup(endpoint, AclRule::Deny, nodes, ctx).await
            }
        }
    }

    async fn setup(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        default_rule: AclRule,
        exceptions: &Vec<GolemIdPattern>,
        ctx: &mut CliCtx,
    ) -> Fallible<CommandResponse> {
        let ack = ctx.prompt_for_acceptance("Are you sure?", None, None);
        if !ack {
            return Ok(CommandResponse::NoOutput);
        }

        let exceptions = exceptions.clone();

        let mut known_peers = endpoint
            .as_golem_net()
            .get_known_peers()
            .await?
            .into_iter()
            .map(|p| p.key)
            .collect::<BTreeSet<_>>();

        let mut connected_peers = endpoint
            .as_golem_net()
            .get_connected_peers()
            .await?
            .into_iter()
            .map(|p| p.node_info.key)
            .collect::<BTreeSet<_>>();

        let mut current_acl = endpoint
            .as_golem_net()
            .acl_status()
            .await?
            .rules
            .into_iter()
            .map(
                |AclRuleItem {
                     identity,
                     node_name: _,
                     rule: _,
                     deadline: _,
                 }| identity,
            )
            .collect::<BTreeSet<_>>();

        let mut b = BTreeSet::new();
        b.append(&mut known_peers);
        b.append(&mut connected_peers);
        b.append(&mut current_acl);

        let candidates: Vec<String> = b.into_iter().collect();
        let nodes = crate::utils::resolve_from_list(candidates, exceptions.clone())?;
        endpoint
            .as_golem_net()
            .acl_setup(default_rule, nodes)
            .await?;
        CommandResponse::object("ACL reset")
    }

    async fn deny(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        ips: &Vec<IpAddr>,
        nodes: &Vec<GolemIdPattern>,
        timeout: i32,
    ) -> Fallible<CommandResponse> {
        let block_ips = future::try_join_all(
            ips.into_iter()
                .cloned()
                .map(|ip| {
                    let endpoint = endpoint.clone();
                    async move {
                        endpoint.as_golem_net().block_ip(ip, timeout).await?;
                        Ok(())
                    }
                })
                .collect::<Vec<_>>(),
        );

        let list_ep = endpoint.clone();
        let nodes = nodes.clone();
        let block_nodes = async {
            let status = endpoint.as_golem_net().acl_status().await?;
            let default_rule = status.default_rule;
            let nodes = match status.default_rule {
                AclRule::Allow => {
                    crate::utils::resolve_from_known_hosts(endpoint.clone(), nodes).await?
                }
                AclRule::Deny => crate::utils::resolve_from_list(
                    status
                        .rules
                        .into_iter()
                        .map(
                            |AclRuleItem {
                                 identity,
                                 node_name: _,
                                 rule: _,
                                 deadline: _,
                             }| identity,
                        )
                        .collect::<Vec<_>>(),
                    nodes,
                )?,
            };

            endpoint
                .as_golem_net()
                .block_node(nodes, timeout)
                .map_ok(
                    |ACLResult {
                         success,
                         exist,
                         message,
                     }| {
                        if !success {
                            println!();
                            eprintln!("Error: {:?}", message.clone().unwrap());
                        }
                        warn_if_exist(default_rule, AclRule::Deny, exist.clone());
                        (success, exist, message)
                    },
                )
                .map_err(failure::Error::from)
                .await
        };

        let (_ips, _nodes) = future::try_join(block_ips, block_nodes).await?;
        list(list_ep, false, false).await
    }

    async fn allow(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        ips: &Vec<IpAddr>,
        nodes: &Vec<GolemIdPattern>,
    ) -> Fallible<CommandResponse> {
        let list_ep = endpoint.clone();

        let allow_ips = future::try_join_all(
            ips.into_iter()
                .cloned()
                .map(|ip| {
                    let endpoint = endpoint.clone();
                    async move {
                        endpoint.as_golem_net().allow_ip(ip, -1).await?;

                        Ok(())
                    }
                })
                .collect::<Vec<_>>(),
        );

        let nodes = nodes.clone();
        let allow_nodes = async {
            let status = endpoint.as_golem_net().acl_status().await?;
            let default_rule = status.default_rule;
            let nodes = match default_rule {
                AclRule::Deny => {
                    crate::utils::resolve_from_known_hosts(endpoint.clone(), nodes).await
                }
                AclRule::Allow => crate::utils::resolve_from_list(
                    status
                        .rules
                        .into_iter()
                        .map(
                            |AclRuleItem {
                                 identity,
                                 node_name: _,
                                 rule: _,
                                 deadline: _,
                             }| identity,
                        )
                        .collect::<Vec<_>>(),
                    nodes,
                ),
            }?;

            endpoint
                .as_golem_net()
                .allow_node(nodes, -1)
                .map_ok(
                    |ACLResult {
                         success,
                         exist,
                         message,
                     }| {
                        if !success {
                            println!();
                            eprintln!("Error: {:?}", message.clone().unwrap());
                        }
                        warn_if_exist(default_rule, AclRule::Allow, exist.clone());
                        (success, exist, message)
                    },
                )
                .map_err(failure::Error::from)
                .await
        };

        let (_ips, _nodes) = future::try_join(allow_ips, allow_nodes).await?;
        list(list_ep, false, false).await
    }
}

struct AclListOutput {
    full: bool,
    ips: Option<Vec<AclRuleItem<IpAddr>>>,
    nodes: AclStatus<String>,
}

impl AclListOutput {
    fn to_response(self) -> CommandResponse {
        let b: Box<dyn FormattedObject> = Box::new(self);

        CommandResponse::FormattedObject(b)
    }
}

impl FormattedObject for AclListOutput {
    fn to_json(&self) -> Fallible<serde_json::Value> {
        Ok(match &self.ips {
            Some(ips) => serde_json::json!({"nodes": self.nodes, "ips": ips}),
            None => serde_json::json!({"nodes": self.nodes}),
        })
    }

    fn print(&self) -> Fallible<()> {
        use prettytable::*;

        if let Some(ref ips) = self.ips {
            println!("Blocked IP Addreses");

            let mut table = create_table(vec!["ip", "valid to"]);

            if ips.is_empty() {
                table.add_row(row!["", ""]);
            }
            for AclRuleItem {
                identity: ip,
                node_name: _,
                rule: _,
                deadline,
            } in ips
            {
                table.add_row(Row::new(vec![
                    Cell::new(&ip.to_string()),
                    Cell::new(
                        &deadline
                            .map(|d| format!("{}", d.with_timezone(&chrono::Local)))
                            .unwrap_or_default(),
                    ),
                ]));
            }
            table.printstd();
        }

        match self.nodes.default_rule {
            AclRule::Deny => println!("Allowed nodes"),
            AclRule::Allow => println!("Blocked nodes"),
        }

        let mut table = create_table(vec!["node", "valid to"]);
        let full = self.full;

        if self.nodes.rules.is_empty() {
            table.add_row(row!["", ""]);
        }

        for AclRuleItem {
            identity,
            node_name: _,
            rule: _,
            deadline,
        } in &self.nodes.rules
        {
            table.add_row(Row::new(vec![
                Cell::new(&format_key(identity, full)),
                Cell::new(
                    &deadline
                        .map(|d| format!("{}", d.with_timezone(&chrono::Local)))
                        .unwrap_or_else(|| "forever".to_string()),
                ),
            ]));
        }

        table.printstd();

        Ok(())
    }
}

async fn list(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    full: bool,
    ip: bool,
) -> Fallible<CommandResponse> {
    if ip {
        let (node_acl, ip_acl) = future::try_join(
            endpoint.as_golem_net().acl_status(),
            endpoint.as_golem_net().acl_ip_status(),
        )
        .await?;
        Ok(AclListOutput {
            nodes: node_acl,
            ips: Some(ip_acl.rules),
            full,
        }
        .to_response())
    } else {
        let node_acl = endpoint.as_golem_net().acl_status().await?;
        Ok(AclListOutput {
            nodes: node_acl,
            ips: None,
            full,
        }
        .to_response())
    }
}

fn warn_if_exist(default_rule: AclRule, direction: AclRule, exist: Option<Vec<String>>) {
    if let Some(mut exist) = exist {
        let adverb = match default_rule {
            AclRule::Deny => match direction {
                AclRule::Deny => "not",
                AclRule::Allow => "already",
            },
            AclRule::Allow => match direction {
                AclRule::Deny => "already",
                AclRule::Allow => "not",
            },
        };

        println!();
        while let Some(node) = exist.pop() {
            eprintln!("Info: {:?} is {} in the list.", node, adverb);
        }
        println!();
    }
}
