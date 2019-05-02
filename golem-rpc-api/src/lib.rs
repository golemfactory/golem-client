#[macro_use]
pub mod rpc;

pub mod comp;
pub mod net;

type Map<K, V> = std::collections::HashMap<K, V>;

pub enum Error {}
