#[macro_use]
pub mod rpc;

pub mod comp;
pub mod core;
pub mod net;
pub mod pay;
pub mod terms;

pub(crate) mod serde;

type Map<K, V> = std::collections::HashMap<K, V>;

pub enum Error {}
