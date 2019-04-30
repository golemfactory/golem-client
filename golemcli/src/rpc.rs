use actix_wamp::{Error, RpcCallRequest, RpcCallResponse, RpcEndpoint, ToArgs};
use futures::future::{Future, IntoFuture};
use futures::{future, prelude::*};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

pub struct Invoker<'a, Inner: RpcEndpoint + ?Sized>(&'a Inner);

pub trait AsInvoker: RpcEndpoint {
    fn as_invoker<'a>(&'a self) -> Invoker<'a, Self>;
}

impl<Inner: RpcEndpoint + Clone> AsInvoker for Inner {
    fn as_invoker<'a>(&'a self) -> Invoker<'a, Self> {
        Invoker(self)
    }
}

impl<'a, Inner: RpcEndpoint + Sized> Invoker<'a, Inner> {
    pub fn rpc_call<'args, Args: ToArgs + 'args, Ret: DeserializeOwned + 'static>(
        &self,
        uri: &'static str,
        args: &Args,
    ) -> impl Future<Item = Ret, Error = Error> + 'static {
        let request = match RpcCallRequest::with_args(uri, args) {
            Ok(resuest) => resuest,
            Err(e) => return future::Either::B(future::err(e)),
        };
        future::Either::A(self.0.rpc_call(request).and_then(
            move |RpcCallResponse { args, .. }| {
                if args.len() != 1 {
                    Err(Error::protocol_err(
                        "invalid rpc response, only 1 args expected",
                    ))
                } else {
                    Ok(serde_json::from_value(args[0].clone()).map_err(move |e| {
                        log::error!("on {} unable to parse: {:?}: {}", uri, args, e);
                        e
                    })?)
                }
            },
        ))
    }

    pub fn rpc_va_call<T: Serialize, Ret: DeserializeOwned + 'static>(
        &self,
        uri: impl Into<Cow<'static, str>>,
        va_args: &Vec<T>,
    ) -> impl Future<Item = Ret, Error = Error> + 'static {
        let uri = uri.into().to_owned();
        let request = match RpcCallRequest::with_va_args(uri.clone(), va_args) {
            Ok(resuest) => resuest,
            Err(e) => return future::Either::B(future::err(e)),
        };
        future::Either::A(self.0.rpc_call(request).and_then(
            move |RpcCallResponse { args, .. }| {
                if args.len() != 1 {
                    Err(Error::protocol_err(
                        "invalid rpc response, only 1 args expected",
                    ))
                } else {
                    Ok(serde_json::from_value(args[0].clone()).map_err(move |e| {
                        log::error!("on {} unable to parse: {:?}: {}", uri, args, e);
                        e
                    })?)
                }
            },
        ))
    }
}
