use crate::context::*;
use futures::Future;
use golem_rpc_api::res::*;
use golem_rpc_api::{core::AsGolemCore, settings::provider};
use structopt::{clap::ArgSettings, StructOpt};

#[derive(StructOpt, Debug)]
pub enum Section {
    #[structopt(name = "_list")]
    ListPresets,
    #[structopt(name = "update")]
    UpdatePresets {
        #[structopt(long = "cores")]
        cpu_cores: Option<u32>,
        #[structopt(long)]
        disk: Option<f64>,
        #[structopt(long)]
        memory: Option<u64>,
    },
    #[structopt(name = "show")]
    Show,
}

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {
        match self {
            Section::ListPresets => futures::future::Either::A(
                endpoint
                    .as_golem_res()
                    .get_hw_presets()
                    .from_err()
                    .and_then(|presets| CommandResponse::object(presets)),
            ),
            Section::Show => futures::future::Either::B(show_presets(endpoint)),
            Section::UpdatePresets {
                cpu_cores,
                disk,
                memory,
            } => futures::future::Either::B(update_presets(endpoint, cpu_cores, disk, memory)),
        }
    }
}

fn show_presets(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
    let active_caps = endpoint
        .as_golem()
        .get_setting::<provider::NumCores>()
        .join3(
            endpoint.as_golem().get_setting::<provider::MaxMemorySize>(),
            endpoint
                .as_golem()
                .get_setting::<provider::MaxResourceSize>(),
        )
        .and_then(|(num_cores, memory, disk)| {
            Ok(HwCaps {
                cpu_cores: num_cores as u32,
                disk,
                memory: memory as u64,
            })
        });
    let max_caps = endpoint.as_golem_res().get_hw_caps();
    let pending_caps = endpoint.as_golem_res().get_hw_preset("custom".into());

    Box::new(get_presets(endpoint)
            .and_then(|r| {
                let columns = vec![
                    "".into(),
                    "active".into(),
                    "pending".into(),
                    "min".into(),
                    "max".into(),
                ];

                let values = vec![
                    serde_json::json!([
                        "cpu_cores",
                        r.active.cpu_cores,
                        r.pending.cpu_cores,
                        r.min.cpu_cores,
                        r.max.cpu_cores
                    ]),
                    serde_json::json!([
                        "disk [kB]",
                        r.active.disk as u64,
                        r.pending.caps.disk as u64,
                        r.min.cpu_cores,
                        r.max.disk
                    ]),
                    serde_json::json!([
                        "memory [Kb]",
                        r.active.memory,
                        r.pending.memory,
                        r.min.memory,
                        r.max.memory
                    ]),
                ];

                Ok(ResponseTable { columns, values }.into())
            }),
    )
    /*  Box::new(endpoint.as_golem().get_setting::<provider::NumCores>().join3(
        endpoint.as_golem().get_setting::<provider::MaxMemorySize>(),
        endpoint.as_golem().get_setting::<provider::MaxResourceSize>()
    ).from_err().and_then(|(num_cores, max_memory_size, max_res_size)| {
        eprintln!("num_cores={}, max_memory_size={}, max_res_size={}", num_cores, max_memory_size, max_res_size);
        CommandResponse::object("ok")
    }))*/
}

fn update_presets(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    cpu_cores: &Option<u32>,
    disk: &Option<f64>,
    memory: &Option<u64>,
) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
    let update = HwPresetUpdate {
        cpu_cores: cpu_cores.as_ref().map(|v| *v),
        disk: disk.as_ref().map(|v| *v),
        memory: memory.as_ref().map(|v| *v),
        name: "custom".to_string(),
    };

    Box::new(
        endpoint
            .as_golem_res()
            .update_hw_preset(update)
            .from_err()
            .and_then(move |n| show_presets(endpoint)),
    )
}

struct HwCapsStatus {
    active: HwCaps,
    pending: HwCaps,
    min: HwCaps,
    max: HwCaps,
}

fn get_presets(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
) -> impl Future<Item = HwCapsStatus, Error = Error> {
    let active_caps = endpoint
        .as_golem()
        .get_setting::<provider::NumCores>()
        .join3(
            endpoint.as_golem().get_setting::<provider::MaxMemorySize>(),
            endpoint
                .as_golem()
                .get_setting::<provider::MaxResourceSize>(),
        )
        .and_then(|(num_cores, memory, disk)| {
            Ok(HwCaps {
                cpu_cores: num_cores as u32,
                disk,
                memory: memory as u64,
            })
        });
    let max_caps = endpoint.as_golem_res().get_hw_caps();
    let pending_caps = endpoint.as_golem_res().get_hw_preset("custom".into());

    active_caps
        .join3(max_caps, pending_caps)
        .from_err()
        .and_then(|(active, max, pending)| {
            Ok(HwCapsStatus {
                active: active,
                pending: pending.caps,
                min: HwCaps {
                    cpu_cores: 1,
                    memory: 1048576,
                    disk: 1048576.0,
                },
                max: max,
            })
        })
}
