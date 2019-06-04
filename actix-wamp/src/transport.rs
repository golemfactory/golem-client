use actix_http::ws;
use actix_web::client::*;
use futures::prelude::*;
use openssl::ssl;
use std::sync::{Arc, Mutex};

pub type ClientError = WsClientError;

/// Websocket over https transport.
pub fn wss(
    host: &str,
    port: u16,
) -> impl Future<
    Item = (
        impl Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>
            + Stream<Item = ws::Frame, Error = ws::ProtocolError>,
        Option<Vec<u8>>,
    ),
    Error = WsClientError,
> + 'static {
    let mut builder = ssl::SslConnector::builder(ssl::SslMethod::tls()).unwrap();
    //builder.set_verify();
    let cert_hash = Arc::new(Mutex::new(None));

    let cert_hash_w = cert_hash.clone();

    builder.set_verify_callback(ssl::SslVerifyMode::NONE, move |internal_check, cert| {
        let hash = cert
            .current_cert()
            .map(|x| x.digest(openssl::hash::MessageDigest::sha1()).unwrap())
            .unwrap();
        log::debug!("internal_check={}, cert={:?}", internal_check, hash);
        *cert_hash_w.lock().unwrap() = Some(hash.to_vec());

        false
    });

    let connector = actix_http::client::Connector::new()
        .ssl(builder.build())
        .finish();

    Client::build()
        .connector(connector)
        .finish()
        .ws(format!("wss://{}:{}", host, port))
        .header("Host", format!("{}:{}", host, port))
        .protocols(&["wamp.2.msgpack"])
        .connect()
        .and_then(move |(resp, framed): (ClientResponse, _)| {
            log::debug!("wss response={:?}", resp);

            Ok((framed, cert_hash.lock().unwrap().take()))
        })
}
