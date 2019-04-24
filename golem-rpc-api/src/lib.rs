pub mod comp;
pub mod net;

type Map<K, V> = std::collections::HashMap<K, V>;

pub enum Error {}

type Result<T> = std::result::Result<T, Error>;
