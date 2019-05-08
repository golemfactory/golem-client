use crate::messages::Dict;
use crate::messages::WampError;
use crate::ErrorKind;
use actix::MailboxError;
use failure::Fail;
use std::borrow::Cow;
use std::error;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "{}", _0)]
    WampError(WampError),

    #[fail(display = "protocol error: {}", _0)]
    ProtocolError(Cow<'static, str>),

    #[fail(display = "mailbox")]
    MailboxError(MailboxError),

    #[fail(display = "connection closed")]
    ConnectionClosed,

    /// Throwed by connection actor in cases when you request action in wrong momment.
    ///
    /// For example:
    ///
    /// * RPC call before WAMP session is opened
    /// * Opening session when she is already opened.
    ///
    #[fail(display = "{}", _0)]
    InvalidState(&'static str),

    #[fail(display = "{}: {}", context, cause)]
    ProcessingError {
        context: Cow<'static, str>,
        cause: Box<dyn error::Error + 'static + Sync + Send>,
    },

    #[fail(display = "{}", _0)]
    WsClientError(String),

    #[fail(display = "{}", _0)]
    ActixProtocolErorr(actix_http::ws::ProtocolError),
}

impl Error {
    #[inline]
    pub fn protocol_err(msg: &'static str) -> Error {
        Error::ProtocolError(Cow::Borrowed(msg))
    }

    pub fn wamp_error(code: ErrorKind, message: String) -> Self {
        let extra = Dict::new();

        Error::WampError(WampError {
            code,
            message,
            extra,
        })
    }

    pub fn from_abort(uri: &str, extra: &Vec<(rmpv::Value, rmpv::Value)>) -> Self {
        let code = ErrorKind::from_uri(uri);
        let extra: Dict = extra
            .into_iter()
            .filter_map(|(k, v)| {
                let key = match k {
                    rmpv::Value::String(key) => key.clone().into_str()?,
                    _ => return None,
                };
                let value = serde_json::to_value(v).ok()?;

                Some((key, value))
            })
            .collect();
        let message = extra
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| code.uri())
            .to_string();

        Error::WampError(WampError {
            code,
            message,
            extra,
        })
    }

    pub fn from_wamp_error_message(uri: &str, args: &rmpv::Value, kwargs: &rmpv::Value) -> Self {
        let code = ErrorKind::from_uri(uri);
        let extra: Dict = kwargs
            .as_map()
            .map(|v| {
                v.into_iter()
                    .filter_map(|(k, v)| {
                        let key = match k {
                            rmpv::Value::String(key) => key.clone().into_str()?,
                            _ => return None,
                        };
                        let value = serde_json::to_value(v).ok()?;

                        Some((key, value))
                    })
                    .collect()
            })
            .unwrap_or_else(|| Dict::new());
        let message = extra
            .get("message")
            .and_then(|v| v.as_str())
            .or_else(|| args[0].as_str())
            .unwrap_or_else(|| code.uri())
            .to_string();

        Error::WampError(WampError {
            code,
            message,
            extra,
        })
    }
}

impl<E: error::Error + 'static + Sync + Send> From<E> for Error {
    fn from(err: E) -> Self {
        Error::ProcessingError {
            context: Cow::Borrowed(""),
            cause: Box::new(err),
        }
    }
}
