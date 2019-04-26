use super::messages::types::*;
use crate::messages::Dict;
use crate::AuthMethod;
use actix::io::WriteHandler;
use actix::prelude::*;
use actix_http::ws;
use futures::prelude::*;
use futures::stream::SplitSink;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Cursor;

fn gen_id() -> u64 {
    use rand::Rng;

    let mut rng = rand::thread_rng();

    rng.gen::<u64>() & 0x0f_ff_ff__ff_ff_ff_ffu64
}

pub struct OpenSession {
    realm_id: String,
    auth_id: Option<String>,
    auth_methods: Vec<Box<dyn AuthMethod>>,
}

impl Message for OpenSession {
    type Result = Result<u64, crate::error::Error>;
}

pub struct RpcCall {
    uri: Cow<'static, str>,
    options: Option<Dict>,
    args: Option<Value>,
    kw_args: Option<Value>,
}

pub struct Connection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>,
{
    writer: actix::io::SinkWrite<W>,
    state: ConnectionState,
}

enum ConnectionState {
    Closed,
    Establishing {
        auth: Vec<Box<dyn AuthMethod>>,
    },
    Authenticating,
    Established {
        session_id: u64,
        pending_calls: HashMap<u64, CallDesc>,
    },
    Failed,
}

struct CallDesc;

impl OpenSession {
    pub fn anonymous(realm_id: String) -> Self {
        OpenSession {
            realm_id,
            auth_id: None,
            auth_methods: Vec::new(),
        }
    }

    pub fn with_auth<A: AuthMethod + 'static>(
        realm_id: String,
        auth_id: String,
        auth_method: A,
    ) -> Self {
        OpenSession {
            realm_id,
            auth_id: Some(auth_id),
            auth_methods: vec![Box::new(auth_method)],
        }
    }
}

impl<W: 'static> Connection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>,
{
    fn new(w: W, ctx: &mut <Self as Actor>::Context) -> Self {
        Connection {
            writer: io::SinkWrite::new(w, ctx),
            state: ConnectionState::Closed,
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

    fn send_call<Args: Serialize>(&mut self, procedure_uri: &str, args: &Args) -> u64 {
        let id = gen_id();

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
        /*let challenge = extra
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
        }*/
    }

    fn handle_welcome(&mut self, session_id: u64, extra: &serde_json::Value) {
        self.state = ConnectionState::Established {
            session_id: session_id,
            pending_calls: HashMap::new(),
        };
        eprintln!("session established");
        let id = self.send_call("golem.password.set", &("123456",));
        eprintln!("set passwrod = {}", id);
        let id = self.send_call("golem.terms.accept", &(true, true));
        eprintln!("accept_terms = {}", id);
    }
}

impl<W: 'static> Actor for Connection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let _ = self.writer.write(ws::Message::Ping("smok".to_string()));
        self.send_hello("golem");
    }
}

impl<W: 'static> StreamHandler<ws::Frame, ws::ProtocolError> for Connection<W>
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

impl<W: 'static> WriteHandler<ws::ProtocolError> for Connection<W> where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>
{
}

pub fn connect<Transport>(transport: Transport) -> Addr<Connection<SplitSink<Transport>>>
where
    Transport: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>
        + Stream<Item = ws::Frame, Error = ws::ProtocolError>
        + 'static,
{
    let (split_sink, split_stream) = transport.split();
    Connection::create(move |ctx| {
        Connection::add_stream(split_stream, ctx);
        Connection::new(split_sink, ctx)
    })
}
