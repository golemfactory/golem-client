use crate::context::*;
use crate::utils::GolemIdPattern;
use futures::{future, prelude::*};
use golem_rpc_api::core::AsGolemCore;
use golem_rpc_api::net::{AclRule, AclRuleItem, AclStatus, AsGolemNet};
use std::borrow::Borrow;
use std::net::IpAddr;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Show current access list status
    #[structopt(name = "list")]
    List {
        ///
        #[structopt(long)]
        ip: bool,
        #[structopt(long)]
        node: bool,
        #[structopt(short="l", long)]
        full: bool,
    },
    #[structopt(name = "deny")]
    Deny {
        #[structopt(long)]
        ip: Vec<IpAddr>,

        #[structopt(long)]
        node: Vec<GolemIdPattern>,

        #[structopt(short = "s", long = "for-secs")]
        for_secs: Option<u32>,
    },

    #[structopt(name = "allow")]
    Allow {
        #[structopt(long)]
        ip: Vec<IpAddr>,
        #[structopt(long)]
        node: Vec<GolemIdPattern>,
    },
}

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            Section::List { full, .. } => Box::new(self.list(endpoint, *full)),
            Section::Deny { ip, node, for_secs } => {
                Box::new(self.deny(endpoint, ip, node, for_secs.map(|s| s as i32).unwrap_or(-1)))
            }
            Section::Allow { ip, node } => Box::new(self.allow(endpoint, ip, node)),
        }
    }

    fn list(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        full : bool
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        endpoint
            .as_golem_net()
            .acl_status()
            .join(endpoint.as_golem_net().acl_ip_status())
            .from_err()
            .and_then(move |(node_acl, ip_acl)| {
                Ok(AclListOutput {
                    nodes: node_acl,
                    ips: ip_acl.rules,
                    full
                }
                .to_response())
            })
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
                                .map(|AclRuleItem(node_id, _, _)| node_id)
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
            .and_then(|(_ips, _nodes)| CommandResponse::object("Updated"))
    }

    fn allow(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        ips: &Vec<IpAddr>,
        nodes: &Vec<GolemIdPattern>,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
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
                                .map(|AclRuleItem(node_id, _, _)| node_id)
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
            .and_then(|(_ips, _nodes)| CommandResponse::object("Updated"))
    }
}

struct AclListOutput {
    full : bool,
    ips: Vec<AclRuleItem<IpAddr>>,
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
        Ok(serde_json::json!({"nodes": self.nodes, "ips": self.ips}))
    }

    fn print(&self) -> Result<(), Error> {
        use prettytable::*;

        println!("Blocked IP Addreses");

        let mut table = create_table(vec!["ip", "valid to"]);

        if self.ips.is_empty() {
            table.add_row(row!["", ""]);
        }
        for AclRuleItem(ip, _, valid_to) in &self.ips {
            table.add_row(Row::new(vec![
                Cell::new(&ip.to_string()),
                Cell::new(
                    &valid_to
                        .map(|d| format!("{}", d.with_timezone(&chrono::Local)))
                        .unwrap_or_default(),
                ),
            ]));
        }
        table.printstd();

        match self.nodes.default_rule {
            AclRule::Deny => println!("Allowed nodes"),
            AclRule::Allow => println!("Blocked nodes"),
        }

        let mut table = create_table(vec!["node", "valid to"]);
        let full = self.full;

        if self.nodes.rules.is_empty() {
            table.add_row(row!["", ""]);
        }


        for AclRuleItem(node, _, valid_to) in &self.nodes.rules {
            table.add_row(Row::new(vec![
                Cell::new(&format_key(node, full)),
                Cell::new(
                    &valid_to
                        .map(|d| format!("{}", d.with_timezone(&chrono::Local)))
                        .unwrap_or_else(|| "forever".to_string()),
                ),
            ]));
        }

        table.printstd();

        Ok(())
    }
}
