use crate::context::*;
use failure::Fallible;
use futures::{future, Future};
use golem_rpc_api::res::*;
use golem_rpc_api::{core::AsGolemCore, settings::provider};
use structopt::{clap::AppSettings, StructOpt};

#[derive(StructOpt, Debug)]
pub enum Section {
    #[structopt(name = "_list")]
    #[structopt(raw(setting = "AppSettings::Hidden"))]
    ListPresets,
    /// Display shared resources info
    #[structopt(name = "show")]
    Show,
    /// Change your provider resources
    #[structopt(name = "update")]
    UpdatePresets {
        #[structopt(long = "cores")]
        cpu_cores: Option<u32>,
        #[structopt(long)]
        disk: Option<f64>,
        #[structopt(long)]
        memory: Option<u64>,
        #[structopt(long)]
        apply: bool,
    },
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
                apply,
            } => futures::future::Either::B(update_presets(
                endpoint, *apply, cpu_cores, disk, memory,
            )),
        }
    }
}

fn none_if_eq<T: Eq>(v1: T, v2: &T) -> Option<T> {
    if v1 == *v2 {
        None
    } else {
        Some(v1)
    }
}

async fn show_presets(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
) -> Fallible<CommandResponse> {
    let r = get_presets(endpoint).await?;

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
            none_if_eq(r.pending.cpu_cores, &r.active.cpu_cores),
            r.min.cpu_cores,
            r.max.cpu_cores
        ]),
        serde_json::json!([
            "disk [kB]",
            r.active.disk as u64,
            none_if_eq(r.pending.disk as u64, &(r.active.disk as u64)),
            r.min.disk,
            r.max.disk
        ]),
        serde_json::json!([
            "memory [kB]",
            r.active.memory,
            none_if_eq(r.pending.memory, &r.active.memory),
            r.min.memory,
            r.max.memory
        ]),
    ];

    Ok(ResponseTable { columns, values }.into())
}

async fn update_presets(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    apply: bool,
    cpu_cores: &Option<u32>,
    disk: &Option<f64>,
    memory: &Option<u64>,
) -> Fallible<CommandResponse> {
    let cpu_cores = cpu_cores.clone();
    let disk = disk.clone();
    let memory = memory.clone();

    let presets = get_presets(endpoint).await?;

    let mut updates = presets.pending.clone();

    if let Some(cpu_cores) = cpu_cores {
        updates.cpu_cores = cpu_cores;
        if cpu_cores < presets.min.cpu_cores || cpu_cores > presets.max.cpu_cores {
            return future::Either::B(future::err(failure::err_msg(format!(
                "cpu cores should be {} >= int >= {}",
                presets.max.cpu_cores, presets.min.cpu_cores
            ))));
        }
    }

    if let Some(memory) = memory {
        updates.memory = memory;
        if memory < presets.min.memory || memory > presets.max.memory {
            return future::Either::B(future::err(failure::err_msg(format!(
                "memory should be {} >= int >= {}",
                presets.max.memory, presets.min.memory
            ))));
        }
    }

    if let Some(disk) = disk {
        eprintln!("disk={}", disk);
        updates.disk = disk;
        if disk < presets.min.disk || disk > presets.max.disk {
            return future::Either::B(future::err(failure::err_msg(format!(
                "disk should be {} >= int >= {}",
                presets.max.disk as u64, presets.min.disk as u64
            ))));
        }
    }

    let update = HwPreset {
        caps: updates.clone(),
        name: "custom".to_string(),
    };

    endpoint.as_golem_res().update_hw_preset(update).await?;

    let changed = presets.active.cpu_cores != updates.cpu_cores
        || presets.active.disk != updates.disk
        || presets.active.memory != updates.memory;

    if changed
        && apply
        && crate::context::prompt_for_acceptance(
            "Changing resources will interrupt performed tasks.\nAre you sure ?",
        )
    {
        eprintln!("Updating resources. please wait");
        let b = endpoint
            .as_golem_res()
            .activate_hw_preset("custom".into(), true)
            .await?;
        CommandResponse::object(b)
    } else {
        if apply && !changed {
            eprintln!("No changes detected");
        }
        show_presets(endpoint).await
    }
}

struct HwCapsStatus {
    active: HwCaps,
    pending: HwCaps,
    min: HwCaps,
    max: HwCaps,
}

fn get_presets(
    endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
) -> impl Future<Item = HwCapsStatus, Error = Error> + 'static {
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
                active,
                pending: pending.caps,
                min: HwCaps {
                    cpu_cores: 1,
                    memory: 1048576,
                    disk: 1048576.0,
                },
                max,
            })
        })
}
