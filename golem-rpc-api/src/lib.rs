pub mod comp;
pub mod net;
pub mod rpc;

type Map<K, V> = std::collections::HashMap<K, V>;

pub enum Error {}

type Result<T> = std::result::Result<T, Error>;
