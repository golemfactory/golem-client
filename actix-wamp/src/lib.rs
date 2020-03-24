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
use futures::prelude::*;

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
    ) -> impl Future<Output = Result<impl RpcEndpoint + PubSubEndpoint + Clone, Error>>
    where
        Transport: Sink<actix_http::ws::Message, Error = actix_http::ws::ProtocolError>
            + Stream<Item = Result<actix_http::ws::Frame, actix_http::ws::ProtocolError>>
            + Unpin
            + 'static,
    {
        let connection = connection::connect(transport);

        connection
            .send(self.msg)
            .then(|r| match r {
                Err(e) => future::err(Error::MailboxError(e)),
                Ok(v) => future::ready(v),
            })
            .and_then(|_| future::ok(connection))
    }

    pub fn create_wss(
        self,
        host: &str,
        port: u16,
    ) -> impl Future<Output = Result<impl RpcEndpoint + PubSubEndpoint + Clone, Error>> {
        wss(host, port)
            .map_err(|e| Error::WsClientError(format!("{}", e)))
            .and_then(move |(transport, _hash)| self.create(transport))
    }
}
