use failure::Fail;

#[macro_use]
pub mod rpc;

pub mod apps;
pub mod comp;
pub mod concent;
pub mod core;
pub mod net;
pub mod pay;
pub mod res;
#[cfg(feature = "settings")]
pub mod settings;
pub mod terms;

mod setup;

pub(crate) mod serde;

pub type Map<K, V> = std::collections::HashMap<K, V>;

pub use setup::{connect_to_app, Net};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "{}", _0)]
    Other(String),

    #[fail(display = "{}", _0)]
    ParseError(#[cause] serde_json::Error),

    #[fail(
        display = "Invalid value for {} (should be {})",
        setting_id, valid_spec
    )]
    ValidationError {
        setting_id: &'static str,
        valid_spec: &'static str,
    },

    #[fail(display = "{}", _0)]
    WampError(#[cause] actix_wamp::Error),

    #[fail(display = "{}", _0)]
    IO(std::io::Error),

    #[fail(display = "{}", _0)]
    Ssl(openssl::error::ErrorStack),
}

impl From<serde_json::error::Error> for Error {
    fn from(json_err: serde_json::Error) -> Self {
        Error::ParseError(json_err)
    }
}

impl From<actix_wamp::Error> for Error {
    fn from(err: actix_wamp::Error) -> Self {
        Error::WampError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err)
    }
}

impl From<openssl::error::ErrorStack> for Error {
    fn from(e: openssl::error::ErrorStack) -> Self {
        Error::Ssl(e)
    }
}
