use actix::io::WriteHandler;
use actix::prelude::*;
use actix_http::body::Body::Bytes;
use actix_http::ws;
use actix_web::client::*;
use actix_web::HttpMessage;
use futures::sink::Sink;
use openssl::ssl;
use serde::Serialize;
use serde_derive::*;
use std::io::Cursor;

const HELLO: u8 = 1;
const WELCOME: u8 = 2;
const ABORT: u8 = 3;
const CHALLENGE: u8 = 4;
const AUTHENTICATE: u8 = 5;
const CALL: u8 = 48;
const RESULT: u8 = 50;

fn get_secret(name: &str) -> Vec<u8> {
    use std::{fs, path::Path};

    fs::read(format!(
        "/home/prekucki/.local/share/golem/default/rinkeby/crossbar/secrets/{}.tck",
        name
    ))
    .unwrap()
}

#[derive(Deserialize, Debug)]
struct WampCraChallenge {
    authid: String,
    authrole: String,
    authmethod: String,
    authprovider: String,
    session: u64,
    nonce: String,
    timestamp: String,
}

struct WampConnection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>,
{
    writer: actix::io::SinkWrite<W>,
    session_id: Option<u64>,
}

impl<W: 'static> WampConnection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>,
{
    fn new(w: W, ctx: &mut <Self as Actor>::Context) -> Self {
        WampConnection {
            writer: io::SinkWrite::new(w, ctx),
            session_id: None,
        }
    }

    fn send_message<M: Serialize>(&mut self, msg: &M) {
        let bytes = rmp_serde::to_vec(msg).unwrap();

        let out_value = rmpv::decode::read_value(&mut Cursor::new(&bytes)).unwrap();
        eprintln!("send message {}", out_value);

        self.writer.write(ws::Message::Binary(bytes.into()));
    }

    fn send_authenticate(&mut self, sign: &str) {
        self.send_message(&(AUTHENTICATE, sign, serde_json::json!({})))
    }

    fn gen_id(&mut self) -> u64 {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        rng.gen::<u64>() & 0x0f_ff_ff__ff_ff_ff_ffu64
    }

    fn send_call<Args: Serialize>(&mut self, procedure_uri: &str, args: &Args) -> u64 {
        let id = self.gen_id();

        self.send_message(&(CALL, id, serde_json::json!({}), procedure_uri, args));

        id
    }

    fn send_hello(&mut self, realm: &str) {
        use rmpv::encode::write_value;
        use rmpv::Value;
        let message = rmpv::Value::Array(vec![
            HELLO.into(),
            realm.into(),
            Value::Map(vec![
                (
                    "roles".into(),
                    Value::Map(vec![
                        ("subscriber".into(), Value::Map(vec![])),
                        ("publisher".into(), Value::Map(vec![])),
                    ]),
                ),
                ("authmethods".into(), Value::Array(vec!["wampcra".into()])),
                ("authid".into(), "golemcli".into()),
            ]),
        ]);
        let mut bytes = Vec::new();

        //let bytes = rmp_serde::to_vec(&message).unwrap();
        write_value(&mut bytes, &message);

        self.writer.write(ws::Message::Binary(bytes.into()));
    }

    fn handle_challenge(&mut self, auth_method: &str, extra: &serde_json::Value) {
        let challenge = extra
            .as_object()
            .and_then(|extra| extra.get("challenge"))
            .and_then(|challenge| challenge.as_str());

        if let Some(challenge) = challenge {
            use hmac::Mac;
            let secret = get_secret("golemcli");
            let mut hmac = hmac::Hmac::<sha2::Sha256>::new_varkey(secret.as_ref()).unwrap();
            hmac.input(challenge.as_bytes());
            let r = hmac.result().code();
            let b = base64::encode(&r);
            eprintln!("result={}", b);
            self.send_authenticate(&b);
        }
    }

    fn handle_welcome(&mut self, session_id: u64, extra: &serde_json::Value) {
        self.session_id = Some(session_id);
        eprintln!("session established");
        let id = self.send_call("golem.password.set", &("123456",));
        eprintln!("set passwrod = {}", id);
        let id = self.send_call("golem.terms.accept", &(true, true));
        eprintln!("accept_terms = {}", id);
    }
}

impl<W: 'static> Actor for WampConnection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let _ = self.writer.write(ws::Message::Ping("smok".to_string()));
        self.send_hello("golem");
    }
}

impl<W: 'static> StreamHandler<ws::Frame, ws::ProtocolError> for WampConnection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>,
{
    fn handle(&mut self, item: ws::Frame, ctx: &mut Self::Context) {
        match item {
            ws::Frame::Binary(Some(bytes)) => {
                let value = rmpv::decode::read_value(&mut Cursor::new(&bytes)).unwrap();
                eprintln!("v={}", value);

                let json_msg: Vec<serde_json::Value> =
                    rmp_serde::from_slice(bytes.as_ref()).unwrap();
                match json_msg[0].as_i64().unwrap() as u8 {
                    CHALLENGE => {
                        self.handle_challenge(json_msg[1].as_str().unwrap(), &json_msg[2]);
                    }
                    WELCOME => {
                        self.handle_welcome(json_msg[1].as_u64().unwrap(), &json_msg[2]);
                    }
                    _ => {}
                }
            }
            _ => eprintln!("h={:?}", item),
        }
    }

    fn started(&mut self, ctx: &mut Self::Context) {}
}

impl<W: 'static> WriteHandler<ws::ProtocolError> for WampConnection<W> where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>
{
}

fn main() -> failure::Fallible<()> {
    flexi_logger::Logger::with_env().start().unwrap();

    let mut sys = System::new("golemcli");
    let mut builder = ssl::SslConnector::builder(ssl::SslMethod::tls())?;

    builder.set_verify(ssl::SslVerifyMode::NONE);

    //builder.set_certificate_file("/home/prekucki/.local/share/golem/default/rinkeby/crossbar/rpc_cert.pem", ssl::SslFiletype::PEM)?;
    //builder.set_private_key_file("/home/prekucki/.local/share/golem/default/rinkeby/crossbar/rpc_key.pem", ssl::SslFiletype::PEM)?;

    let connector = actix_http::client::Connector::new()
        .ssl(builder.build())
        .finish();

    let c = Client::build().connector(connector).finish();
    let out = c
        .ws("wss://127.0.0.1:61000")
        .header("Host", "127.0.0.1:61000")
        .protocols(&[/*"wamp.2.json",*/ "wamp.2.msgpack"])
        .connect()
        .map_err(|e| eprintln!("{}", e));

    let _ = sys
        .block_on(out.and_then(|(resp, framed): (ClientResponse, _)| {
            let v = resp.headers().get("sec-websocket-protocol");
            eprintln!("proto={:?}", v.unwrap());
            eprintln!("result = {:?}", resp);
            let (w, r) = framed.split();
            let addr = WampConnection::create(move |ctx| {
                WampConnection::add_stream(r, ctx);
                WampConnection::new(w, ctx)
            });

            Ok(addr)
        }))
        .unwrap();

    let _ = sys.run();

    Ok(println!("Hello, world!"))
}
