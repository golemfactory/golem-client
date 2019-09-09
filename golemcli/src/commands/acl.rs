use crate::context::*;
use crate::utils::GolemIdPattern;
use futures::{future, prelude::*};
use golem_rpc_api::core::AsGolemCore;
use golem_rpc_api::net::{AclRule, AclRuleItem, AclStatus, AsGolemNet, NodeInfo, PeerInfo};
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
    pub fn run(
        &self,
        ctx: &mut CliCtx,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            Section::List { full, ip, .. } => Box::new(list(endpoint, *full, *ip)),
            Section::Deny { ip, node, for_secs } => {
                Box::new(self.deny(endpoint, ip, node, for_secs.map(|s| s as i32).unwrap_or(-1)))
            }
            Section::Allow { ip, node } => Box::new(self.allow(endpoint, ip, node)),
            Section::Setup(Setup::AllExcept { nodes }) => {
                Box::new(self.setup(endpoint, AclRule::Allow, nodes, ctx))
            }
            Section::Setup(Setup::OnlyListed { nodes }) => {
                Box::new(self.setup(endpoint, AclRule::Deny, nodes, ctx))
            }
        }
    }

    fn setup(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        default_rule: AclRule,
        exceptions: &Vec<GolemIdPattern>,
        ctx: &mut CliCtx,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let ack = ctx.prompt_for_acceptance("Are you sure?", None, None);
        if !ack {
            return future::Either::A(future::ok(CommandResponse::NoOutput));
        }

        let exceptions = exceptions.clone();

        let known_peers = endpoint
            .as_golem_net()
            .get_known_peers()
            .from_err()
            .and_then(|peers: Vec<NodeInfo>| {
                Ok(peers.into_iter().map(|p| p.key).collect::<BTreeSet<_>>())
            });
        let connected_peers = endpoint
            .as_golem_net()
            .get_connected_peers()
            .from_err()
            .and_then(|peers: Vec<PeerInfo>| {
                Ok(peers
                    .into_iter()
                    .map(|p| p.node_info.key)
                    .collect::<BTreeSet<_>>())
            });
        let current_acl = endpoint.as_golem_net().acl_status().from_err().and_then(
            |status: AclStatus<String>| {
                Ok(status
                    .rules
                    .into_iter()
                    .map(|AclRuleItem{node_id, node_name: _, rule: _, deadline: _}| node_id)
                    .collect::<BTreeSet<_>>())
            },
        );
        future::Either::B(
            crate::utils::resolve_from_list(
                known_peers.join3(connected_peers, current_acl).and_then(
                    |(mut l1, mut l2, mut l3): (
                        BTreeSet<String>,
                        BTreeSet<String>,
                        BTreeSet<String>,
                    )| {
                        l1.append(&mut l2);
                        l1.append(&mut l3);
                        Ok(l1.into_iter().collect::<Vec<_>>())
                    },
                ),
                exceptions.clone(),
            )
            .and_then(move |nodes| {
                endpoint
                    .as_golem_net()
                    .acl_setup(default_rule, nodes)
                    .from_err()
            })
            .and_then(|()| CommandResponse::object("ACL reset")),
        )
    }

    fn deny(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        ips: &Vec<IpAddr>,
        nodes: &Vec<GolemIdPattern>,
        timeout: i32,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let block_ips = future::join_all(
            ips.into_iter()
                .cloned()
                .map(|ip| {
                    endpoint
                        .as_golem_net()
                        .block_ip(ip, timeout)
                        .and_then(|_| Ok(()))
                })
                .collect::<Vec<_>>(),
        );

        let list_ep = endpoint.clone();
        let nodes = nodes.clone();
        let block_nodes = endpoint
            .as_golem_net()
            .acl_status()
            .from_err()
            .and_then(move |status| {
                match status.default_rule {
                    AclRule::Allow => future::Either::B(crate::utils::resolve_from_known_hosts(
                        endpoint.clone(),
                        nodes,
                    )),
                    AclRule::Deny => future::Either::A(crate::utils::resolve_from_list(
                        future::ok(
                            status
                                .rules
                                .into_iter()
                                .map(|AclRuleItem{node_id, node_name: _, rule: _, deadline: _}| node_id)
                                .collect::<Vec<_>>(),
                        ),
                        nodes,
                    )),
                }
                .and_then(move |nodes| {
                    future::join_all(
                        nodes
                            .into_iter()
                            .map(|node_id: String| {
                                endpoint
                                    .as_golem_net()
                                    .block_node(node_id, timeout)
                                    .from_err()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .and_then(|_| Ok(()))
                })
            });

        block_ips
            .from_err()
            .join(block_nodes)
            .and_then(move |(_ips, _nodes)| list(list_ep, false, false))
    }

    fn allow(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        ips: &Vec<IpAddr>,
        nodes: &Vec<GolemIdPattern>,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        let list_ep = endpoint.clone();

        let allow_ips = future::join_all(
            ips.into_iter()
                .cloned()
                .map(|ip| {
                    endpoint
                        .as_golem_net()
                        .allow_ip(ip, -1)
                        .from_err()
                        .and_then(|_| Ok(()))
                })
                .collect::<Vec<_>>(),
        );

        let nodes = nodes.clone();
        let allow_nodes = endpoint
            .as_golem_net()
            .acl_status()
            .from_err()
            .and_then(move |status| {
                match status.default_rule {
                    AclRule::Deny => future::Either::B(crate::utils::resolve_from_known_hosts(
                        endpoint.clone(),
                        nodes,
                    )),
                    AclRule::Allow => future::Either::A(crate::utils::resolve_from_list(
                        future::ok(
                            status
                                .rules
                                .into_iter()
                                .map(|AclRuleItem{node_id, node_name: _, rule: _, deadline: _}| node_id)
                                .collect::<Vec<_>>(),
                        ),
                        nodes,
                    )),
                }
                .and_then(move |nodes| {
                    future::join_all(
                        nodes
                            .into_iter()
                            .map(|node_id: String| {
                                endpoint.as_golem_net().allow_node(node_id, -1).from_err()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .and_then(|_| Ok(()))
                })
            });

        allow_ips
            .join(allow_nodes)
            .from_err()
            .and_then(move |(_ips, _nodes)| list(list_ep, false, false))
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
    fn to_json(&self) -> Result<serde_json::Value, Error> {
        Ok(match &self.ips {
            Some(ips) => serde_json::json!({"nodes": self.nodes, "ips": ips}),
            None => serde_json::json!({"nodes": self.nodes}),
        })
    }

    fn print(&self) -> Result<(), Error> {
        use prettytable::*;

        if let Some(ref ips) = self.ips {
            println!("Blocked IP Addreses");

            let mut table = create_table(vec!["ip", "valid to"]);

            if ips.is_empty() {
                table.add_row(row!["", ""]);
            }
            for AclRuleItem{node_id: ip, node_name: _, rule: _, deadline} in ips {
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

        for AclRuleItem{node_id, node_name: _, rule: _, deadline} in &self.nodes.rules {
            table.add_row(Row::new(vec![
                Cell::new(&format_key(node_id, full)),
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

fn list(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    full: bool,
    ip: bool,
) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
    if ip {
        future::Either::A(
            endpoint
                .as_golem_net()
                .acl_status()
                .join(endpoint.as_golem_net().acl_ip_status())
                .from_err()
                .and_then(move |(node_acl, ip_acl)| {
                    Ok(AclListOutput {
                        nodes: node_acl,
                        ips: Some(ip_acl.rules),
                        full,
                    }
                    .to_response())
                }),
        )
    } else {
        future::Either::B(endpoint.as_golem_net().acl_status().from_err().and_then(
            move |node_acl| {
                Ok(AclListOutput {
                    nodes: node_acl,
                    ips: None,
                    full,
                }
                .to_response())
            },
        ))
    }
}
