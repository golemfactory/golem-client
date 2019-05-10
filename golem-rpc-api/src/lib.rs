use failure::Fail;

#[macro_use]
pub mod rpc;

pub mod comp;
pub mod core;
pub mod net;
pub mod pay;
#[cfg(feature="settings")]
pub mod settings;
pub mod terms;

pub(crate) mod serde;

type Map<K, V> = std::collections::HashMap<K, V>;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "{}", _0)]
    Other(String),
    #[fail(display = "{}", _0)]
    ParseError(#[cause] serde_json::Error)
}


impl From<serde_json::error::Error> for Error {
    fn from(json_err: serde_json::Error) -> Self {
        Error::ParseError(json_err)
    }
}
