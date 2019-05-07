use actix_http::ws;
use actix_web::client::*;
use futures::prelude::*;
use openssl::ssl;

/// Websocket over https transport.
pub fn wss(
    host: &str,
    port: u16,
) -> impl Future<
    Item = impl Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>
               + Stream<Item = ws::Frame, Error = ws::ProtocolError>,
    Error = WsClientError,
> + 'static {
    let mut builder = ssl::SslConnector::builder(ssl::SslMethod::tls()).unwrap();
    //builder.set_verify();
    builder.set_verify_callback(ssl::SslVerifyMode::NONE, |internal_check, cert| {
        log::debug!(
            "internal_check={}, cert={:?}",
            internal_check,
            cert.current_cert()
                .map(|x| x.digest(openssl::hash::MessageDigest::sha1()).unwrap())
        );

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
        .and_then(|(resp, framed): (ClientResponse, _)| {
            log::debug!("wss response={:?}", resp);

            Ok(framed)
        })
}
