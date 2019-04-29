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
}

impl<E: error::Error + 'static + Sync + Send> From<E> for Error {
    fn from(err: E) -> Self {
        Error::ProcessingError {
            context: Cow::Borrowed(""),
            cause: Box::new(err),
        }
    }
}
