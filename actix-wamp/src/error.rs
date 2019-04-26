use crate::messages::WampError;
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

    #[fail(display = "{}: {}", context, cause)]
    ProcessingError {
        context: Cow<'static, str>,
        cause: Box<dyn error::Error + 'static + Sync + Send>,
    },
}

impl Error {
    pub fn protocol_err(msg: &'static str) -> Error {
        Error::ProtocolError(Cow::Borrowed(msg))
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
