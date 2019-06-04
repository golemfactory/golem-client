use crate::error::Error;
use crate::messages::{Dict, WampError};
use actix::Message;
use futures::prelude::*;
use futures::sync::mpsc;
use serde_json::Value;
use std::borrow::Cow;

pub struct WampMessage {
    pub args: Vec<Value>,
    pub kw_args: Option<Dict>,
}

pub trait PubSubEndpoint {
    type Events: Stream<Item = WampMessage, Error = Error>;

    fn subscribe(&mut self, uri: &str) -> Self::Events;
}

pub struct Unsubscribe {
    pub subscription_id: u64,
}

impl Message for Unsubscribe {
    type Result = Result<(), Error>;
}

pub struct Subscribe {
    pub topic: Cow<'static, str>,
}

impl Message for Subscribe {
    type Result = Result<Subscription, Error>;
}

pub struct Subscription {
    subscription_id: u64,
    stream: mpsc::UnboundedReceiver<Result<WampMessage, WampError>>,
    connection: actix::Recipient<Unsubscribe>,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let subscription_id = self.subscription_id;
        let _ = self.connection.do_send(Unsubscribe { subscription_id });
    }
}

impl Stream for Subscription {
    type Item = WampMessage;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        match self.stream.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(Err(e)))) => Err(Error::WampError(e)),
            Ok(Async::Ready(Some(Ok(message)))) => Ok(Async::Ready(Some(message))),
            Err(_) => Err(Error::ConnectionClosed),
        }
    }
}
