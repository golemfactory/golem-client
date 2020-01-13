use crate::error::Error;
use crate::messages::{Dict, WampError};
use actix::Message;
use futures::channel::mpsc;
use futures::prelude::*;
use futures::task::{Context, Poll};
use futures::StreamExt;
use serde_json::Value;
use std::borrow::Cow;
use std::pin::Pin;

#[derive(Debug)]
pub struct WampMessage {
    pub args: Vec<Value>,
    pub kw_args: Option<Dict>,
}

pub trait PubSubEndpoint {
    type Events: Stream<Item = Result<WampMessage, Error>>;

    fn subscribe(&self, uri: &str) -> Self::Events;
}

pub struct Unsubscribe {
    pub subscription_id: u64,
}

impl Message for Unsubscribe {
    type Result = ();
}

pub struct Subscribe {
    pub topic: Cow<'static, str>,
}

impl Message for Subscribe {
    type Result = Result<Subscription, Error>;
}

pub struct Subscription {
    pub(crate) subscription_id: u64,
    pub(crate) stream: mpsc::UnboundedReceiver<Result<WampMessage, WampError>>,
    pub(crate) connection: actix::Recipient<Unsubscribe>,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let subscription_id = self.subscription_id;
        let _ = self.connection.do_send(Unsubscribe { subscription_id });
    }
}

impl Stream for Subscription {
    type Item = Result<WampMessage, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream
            .poll_next_unpin(cx)
            .map(|v| v.map(|v| v.map_err(Error::WampError)))
    }
}
