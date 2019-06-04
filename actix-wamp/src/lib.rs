pub(crate) mod args;
mod auth;
pub(crate) mod connection;
mod error;
mod messages;
pub(crate) mod pubsub;
mod transport;

pub use messages::ErrorKind;

pub use auth::wampcra::challenge_response_auth;
pub use auth::AuthMethod;
pub use error::Error;
pub use messages::WampError;
pub use pubsub::PubSubEndpoint;
pub use transport::{wss, ClientError};

pub use args::{RpcCallRequest, RpcCallResponse, RpcEndpoint, ToArgs};
use futures::Future;

pub struct SessionBuilder {
    msg: connection::OpenSession,
}

impl SessionBuilder {
    #[inline]
    pub fn anonymous(realm_id: String) -> Self {
        SessionBuilder {
            msg: connection::OpenSession::anonymous(realm_id),
        }
    }

    #[inline]
    pub fn with_auth<A: AuthMethod + 'static + Send>(
        realm_id: impl Into<String>,
        auth_id: impl Into<String>,
        auth_method: A,
    ) -> Self {
        SessionBuilder {
            msg: connection::OpenSession::with_auth(realm_id.into(), auth_id.into(), auth_method),
        }
    }

    pub fn create<Transport>(
        self,
        transport: Transport,
    ) -> impl Future<Item = impl RpcEndpoint + PubSubEndpoint + Clone, Error = Error>
    where
        Transport: futures::Sink<
                SinkItem = actix_http::ws::Message,
                SinkError = actix_http::ws::ProtocolError,
            > + futures::Stream<
                Item = actix_http::ws::Frame,
                Error = actix_http::ws::ProtocolError,
            > + 'static,
    {
        use futures::prelude::*;

        let connection = connection::connect(transport);

        connection
            .send(self.msg)
            .then(|r| match r {
                Err(e) => Err(Error::MailboxError(e)),
                Ok(v) => v,
            })
            .and_then(|_| Ok(connection))
    }

    pub fn create_wss(
        self,
        host: &str,
        port: u16,
    ) -> impl Future<Item = impl RpcEndpoint + PubSubEndpoint + Clone, Error = Error> {
        wss(host, port)
            .map_err(|e| Error::WsClientError(format!("{}", e)))
            .and_then(move |(transport, _hash)| self.create(transport))
    }
}
