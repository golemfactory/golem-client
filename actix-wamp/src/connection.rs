use super::messages::types::*;
use crate::args::*;
use crate::error::Error;
use crate::messages::{Dict, WampError};
use crate::pubsub;
use crate::{AuthMethod, ErrorKind};
use actix::io::WriteHandler;
use actix::prelude::*;
use actix_http::ws;
use futures::prelude::*;
use futures::stream::SplitSink;
use futures::sync::mpsc;
use futures::unsync::oneshot;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Cursor;

use crate::args::RpcEndpoint;

use crate::pubsub::WampMessage;
use futures::{Flatten, FlattenStream, IntoStream};
use std::convert::TryInto;
//use crate::messages::types as msg_type;

fn gen_id() -> u64 {
    use rand::Rng;

    let mut rng = rand::thread_rng();

    rng.gen::<u64>() & 0x1f_ff_ff__ff_ff_ff_ffu64
}

pub struct OpenSession {
    realm_id: String,
    auth_id: Option<String>,
    auth_methods: Vec<Box<dyn AuthMethod + Send + 'static>>,
}

impl Message for OpenSession {
    type Result = Result<u64, crate::error::Error>;
}

pub struct Connection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>,
{
    writer: actix::io::SinkWrite<W>,
    state: ConnectionState,
}

type SubSender = mpsc::UnboundedSender<Result<pubsub::WampMessage, WampError>>;

enum ConnectionState {
    Closed,
    Establishing {
        auth: Vec<Box<dyn AuthMethod + Send + 'static>>,
        auth_id: Option<String>,
        tx: Option<oneshot::Sender<Result<u64, Error>>>,
    },
    Authenticating {
        tx: oneshot::Sender<Result<u64, Error>>,
    },
    Established {
        #[allow(dead_code)]
        session_id: u64,
        pending_calls: HashMap<u64, CallDesc>,
        subscribers: HashMap<u64, SubSender>,
        pending_subscriptions: HashMap<u64, oneshot::Sender<Result<u64, Error>>>,
    },
    Failed,
}

struct CallDesc {
    tx: oneshot::Sender<Result<RpcCallResponse, Error>>,
}

impl OpenSession {
    pub fn anonymous(realm_id: String) -> Self {
        OpenSession {
            realm_id,
            auth_id: None,
            auth_methods: Vec::new(),
        }
    }

    pub fn with_auth<A: AuthMethod + 'static + Send>(
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

    fn send_message<M: Serialize>(&mut self, msg: &M) -> Result<(), Error> {
        let bytes = rmp_serde::to_vec(&serde_json::to_value(msg)?)?;
        //let bytes = rmp_serde::to_vec(&serde_json::to_value(msg)?)?;

        if log::log_enabled!(log::Level::Debug) {
            let out_value = rmpv::decode::read_value(&mut Cursor::new(&bytes)).unwrap();
            log::debug!("send message {}", out_value);
        }

        self.writer
            .write(ws::Message::Binary(bytes.into()))
            .map(|_| ())
            .map_err(|e| Error::ActixProtocolErorr(e))
    }

    fn handle_challenge(&mut self, auth_method: &str, extra: &Dict) -> Result<(), Error> {
        let (auth_methods, auth_id, tx) = match &mut self.state {
            ConnectionState::Establishing {
                auth, auth_id, tx, ..
            } => match auth_id {
                Some(auth_id) => (auth, auth_id.as_str(), tx),
                None => {
                    return Err(Error::protocol_err(
                        "unexpected challenge on anonymous handshake",
                    ))
                }
            },
            _ => {
                return Err(Error::wamp_error(
                    ErrorKind::OptionNotAllowed,
                    "invalid connection state".into(),
                ))
            }
        };

        for auth in auth_methods {
            if auth.auth_method() == auth_method {
                let (signature, extra) = auth.challenge(auth_id, extra)?;
                let tx = tx.take().unwrap();
                self.state = ConnectionState::Authenticating { tx };
                self.send_message(&(AUTHENTICATE, signature, extra))?;
                return Ok(());
            }
        }

        self.state = ConnectionState::Failed;
        Err(Error::protocol_err("unexpected auth method received"))
    }

    fn handle_welcome(&mut self, session_id: u64, extra: &serde_json::Value) -> Result<(), Error> {
        log::debug!("got welcome: {}", extra);
        let old_state = std::mem::replace(
            &mut self.state,
            ConnectionState::Established {
                session_id,
                pending_calls: HashMap::new(),
                subscribers: HashMap::new(),
                pending_subscriptions: HashMap::new(),
            },
        );
        match old_state {
            ConnectionState::Establishing { tx, .. } => {
                let _ = tx.unwrap().send(Ok(session_id));
            }
            ConnectionState::Authenticating { tx, .. } => {
                let _ = tx.send(Ok(session_id));
            }
            _ => (),
        };

        Ok(())
    }

    fn pending_calls(&mut self) -> Result<&mut HashMap<u64, CallDesc>, Error> {
        match &mut self.state {
            ConnectionState::Established { pending_calls, .. } => Ok(pending_calls),
            _ => Err(Error::InvalidState("session is closed or pending")),
        }
    }

    fn handle_result(&mut self, call_id: u64, args: Option<rmpv::Value>) -> Result<(), Error> {
        if let Some(CallDesc { tx }) = self.pending_calls()?.remove(&call_id) {
            let args = args
                .and_then(|args| serde_json::to_value(args).ok())
                .and_then(|args| args.as_array().cloned())
                .unwrap_or_default();

            let _ = tx.send(Ok(RpcCallResponse {
                args,
                kw_args: None,
            }));
        }
        Ok(())
    }

    #[inline]
    fn subscribers(&mut self) -> Result<&mut HashMap<u64, SubSender>, Error> {
        match &mut self.state {
            ConnectionState::Established { subscribers, .. } => Ok(subscribers),
            _ => Err(Error::InvalidState("session is closed or pending")),
        }
    }

    fn handle_event(
        &mut self,
        sub_id: u64,
        pub_id: u64,
        args: Option<&rmpv::Value>,
        kwargs: Option<&rmpv::Value>,
    ) -> Result<(), Error> {
        if let Some(tx) = self.subscribers()?.get_mut(&sub_id) {
            let args = args
                .and_then(|args| serde_json::to_value(args).ok())
                .and_then(|args| args.as_array().cloned())
                .unwrap_or_default();

            // TODO: catch kw_args

            let _ = tx.send(Ok(WampMessage {
                args,
                kw_args: None,
            }));
        } else {
            log::warn!("unhandled event: subscription_id={}", sub_id);
        }
        Ok(())
    }

    fn handle_abort(
        &mut self,
        error_uri: &str,
        extra: &Vec<(rmpv::Value, rmpv::Value)>,
    ) -> Result<(), Error> {
        match std::mem::replace(&mut self.state, ConnectionState::Failed) {
            ConnectionState::Authenticating { tx } => {
                // TODO: log error
                let _ = tx.send(Err(Error::from_abort(error_uri, extra)));
            }
            ConnectionState::Established { pending_calls, .. } => {
                for (_call_id, desc) in pending_calls {
                    // TODO: log error
                    let _ = desc.tx.send(Err(Error::from_abort(error_uri, extra)));
                }
            }
            _ => (),
        }

        Ok(())
    }
    // [
    //      ERROR,
    //      REQUEST.Type|int,
    //      REQUEST.Request|id,
    //      Details|dict,
    //      Error|uri,
    //      Arguments|list,
    // ArgumentsKw|dict]
    fn handle_error(
        &mut self,
        request_type: u64,
        request_id: u64,
        details: &rmpv::Value,
        error_uri: &str,
        args: &rmpv::Value,
        kwargs: &rmpv::Value,
    ) -> Result<(), Error> {
        match request_type.try_into()? {
            CALL => self.handle_error_call(request_id, details, error_uri, args, kwargs),
            _ => Ok(()),
        }
    }

    fn handle_error_call(
        &mut self,
        request_id: u64,
        _details: &rmpv::Value,
        error_uri: &str,
        args: &rmpv::Value,
        kwargs: &rmpv::Value,
    ) -> Result<(), Error> {
        log::info!("handle call: {}", request_id);
        let calls = match &mut self.state {
            ConnectionState::Established { pending_calls, .. } => pending_calls,
            _ => return Ok(()),
        };
        if let Some(desc) = calls.remove(&request_id) {
            let _ = desc
                .tx
                .send(Err(Error::from_wamp_error_message(error_uri, args, kwargs)));
        } else {
            log::error!("invalid id");
        }
        Ok(())
    }
}

impl<W: 'static> Actor for Connection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>,
{
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let _ = self.writer.write(ws::Message::Ping("smok".to_string()));
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        log::debug!("connection stopped");
    }
}

impl<W: 'static> StreamHandler<ws::Frame, ws::ProtocolError> for Connection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>,
{
    fn handle(&mut self, item: ws::Frame, _ctx: &mut Self::Context) {
        match item {
            ws::Frame::Binary(Some(bytes)) => {
                let value = rmpv::decode::read_value(&mut Cursor::new(&bytes)).unwrap();
                log::trace!("got message ={}", value);

                match value[0].as_i64().unwrap() as u8 {
                    WELCOME => {
                        let _ = self.handle_welcome(
                            value[1].as_u64().unwrap(),
                            &serde_json::to_value(&value[2].as_map()).unwrap(),
                        );
                    }
                    ABORT => {
                        // [3, {"message": "WAMP-CRA signature is invalid"}, "wamp.error.not_authorized"]
                        let _ = self
                            .handle_abort(value[2].as_str().unwrap(), value[1].as_map().unwrap());
                    }

                    CHALLENGE => {
                        self.handle_challenge(
                            value[1].as_str().unwrap(),
                            &serde_json::to_value(&value[2])
                                .unwrap()
                                .as_object()
                                .unwrap(),
                        )
                        .unwrap_or_else(|e| {
                            log::error!("auth mathod failed with: {}", e);
                        });
                    }
                    RESULT => {
                        let _ =
                            self.handle_result(value[1].as_u64().unwrap(), Some(value[3].clone()));
                    }
                    EVENT => {
                        //[EVENT, SUBSCRIBED.Subscription|id, PUBLISHED.Publication|id, Details|dict, PUBLISH.Arguments|list, PUBLISH.ArgumentKw|dict]
                        let subscription_id = value[1].as_u64().unwrap();
                        let publication_id = value[1].as_u64().unwrap();
                        let args = value.as_array().and_then(|a| a.get(4));
                        let kwargs = value.as_array().and_then(|a| a.get(5));
                        self.handle_event(subscription_id, publication_id, args, kwargs)
                            .unwrap();
                    }
                    ERROR => {
                        // There are 2 formats
                        // [
                        //      ERROR,
                        //      REQUEST.Type|int,
                        //      REQUEST.Request|id,
                        //      Details|dict,
                        //      Error|uri,
                        //      Arguments|list,
                        // ArgumentsKw|dict]
                        log::trace!("got error");
                        let _ = self.handle_error(
                            value[1].as_u64().unwrap(),
                            value[2].as_u64().unwrap(),
                            &value[3],
                            value[4].as_str().unwrap(),
                            &value[5],
                            &value[6],
                        );
                    }
                    _ => {}
                }
            }
            _ => log::debug!("h={:?}", item),
        }
    }

    fn started(&mut self, _ctx: &mut Self::Context) {
        // TODO
    }
}

impl<W: 'static> WriteHandler<ws::ProtocolError> for Connection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>,
{
    fn error(&mut self, err: ws::ProtocolError, _ctx: &mut Self::Context) -> Running {
        log::error!("protocol error: {}", err);
        self.state = ConnectionState::Failed;
        Running::Stop
    }
}

impl<W> Handler<OpenSession> for Connection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError> + 'static,
{
    type Result = ActorResponse<Self, u64, crate::error::Error>;

    fn handle(
        &mut self,
        OpenSession {
            realm_id,
            auth_id,
            auth_methods,
        }: OpenSession,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        use crate::messages::{HelloSpec, Role, RoleDesc};

        // check state
        match self.state {
            ConnectionState::Closed => (),
            _ => {
                return ActorResponse::reply(Err(Error::InvalidState(
                    "session is already opened or operation pending",
                )))
            }
        }

        let (tx, rx) = futures::unsync::oneshot::channel();
        let auth_methods_id = auth_methods.iter().map(|method| method.auth_method());

        let auth_id_ref = match &auth_id {
            Some(v) => Some(v.as_str()),
            None => None,
        };

        let _ = self.send_message(&(
            HELLO,
            realm_id,
            HelloSpec {
                roles: vec![(Role::Caller, RoleDesc::default())]
                    .into_iter()
                    .collect(),
                auth_methods: auth_methods_id.collect(),
                authid: auth_id_ref,
            },
        ));
        self.state = ConnectionState::Establishing {
            auth: auth_methods,
            auth_id,
            tx: Some(tx),
        };

        ActorResponse::r#async(
            rx.then(|r| match r {
                Err(_e) => Err(Error::ConnectionClosed),
                Ok(resp) => resp,
            })
            .into_actor(self),
        )
    }
}

impl<W> Handler<RpcCallRequest> for Connection<W>
where
    W: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError> + 'static,
{
    type Result = ActorResponse<Self, RpcCallResponse, crate::error::Error>;

    fn handle(
        &mut self,
        RpcCallRequest {
            uri,
            options,
            args,
            kw_args,
        }: RpcCallRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let pending_calls = match &mut self.state {
            ConnectionState::Established { pending_calls, .. } => pending_calls,
            _ => {
                return ActorResponse::reply(Err(Error::InvalidState(
                    "session is closed or pending",
                )))
            }
        };

        // generate rpc-call-id. spec says that is should be random.
        let id = {
            let mut id = gen_id();

            while pending_calls.contains_key(&id) {
                id = gen_id()
            }

            id
        };
        let (tx, rx) = oneshot::channel();

        pending_calls.insert(id, CallDesc { tx });

        let result = match (args, kw_args) {
            (None, None) => self.send_message(&(CALL, id, options.unwrap_or_default(), uri)),
            (Some(args), None) => {
                self.send_message(&(CALL, id, options.unwrap_or_default(), uri, args))
            }
            (args, Some(kw_args)) => self.send_message(&(
                CALL,
                id,
                options.unwrap_or_default(),
                uri,
                args.unwrap_or_else(|| serde_json::json!([])),
                kw_args,
            )),
        };

        if let Err(e) = result {
            return ActorResponse::reply(Err(e));
        }

        ActorResponse::r#async(
            rx.then(|r| match r {
                Err(_) => Err(Error::ConnectionClosed),
                Ok(resp) => resp,
            })
            .into_actor(self),
        )
    }
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

impl<Transport> RpcEndpoint for Addr<Connection<SplitSink<Transport>>>
where
    Transport: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>
        + Stream<Item = ws::Frame, Error = ws::ProtocolError>
        + 'static,
{
    type Response = Box<dyn Future<Item = RpcCallResponse, Error = Error> + 'static>;

    fn rpc_call(&self, request: RpcCallRequest) -> Self::Response {
        Box::new(self.send(request).then(|resp| match resp {
            Err(e) => Err(Error::MailboxError(e)),
            Ok(v) => v,
        }))
    }
}

impl<Transport> super::PubSubEndpoint for Addr<Connection<SplitSink<Transport>>>
where
    Transport: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>
        + Stream<Item = ws::Frame, Error = ws::ProtocolError>
        + 'static,
{
    type Events =
        FlattenStream<Flatten<Request<Connection<SplitSink<Transport>>, crate::pubsub::Subscribe>>>;

    fn subscribe(&mut self, uri: &str) -> Self::Events {
        self.send(crate::pubsub::Subscribe {
            topic: Cow::Owned(uri.into()),
        })
        .flatten()
        .flatten_stream()
    }
}

impl<Transport> Handler<crate::pubsub::Subscribe> for Connection<SplitSink<Transport>>
where
    Transport: Sink<SinkItem = ws::Message, SinkError = ws::ProtocolError>
        + Stream<Item = ws::Frame, Error = ws::ProtocolError>
        + 'static,
{
    type Result = Result<crate::pubsub::Subscription, Error>;

    fn handle(&mut self, msg: crate::pubsub::Subscribe, ctx: &mut Self::Context) -> Self::Result {
        //let (tx, rx) = mpsc::unbounded();

        unimplemented!()
    }
}
