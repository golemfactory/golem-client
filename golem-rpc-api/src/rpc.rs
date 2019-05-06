use actix_wamp::{Error, RpcCallRequest, RpcCallResponse, RpcEndpoint, ToArgs};
pub use futures::future::{self, Future};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::borrow::Cow;

pub mod wamp {
    pub use actix_wamp::{Error, RpcCallRequest, RpcCallResponse, RpcEndpoint, ToArgs};
    pub use futures::Future;
}

pub struct Invoker<'a, Inner: RpcEndpoint + ?Sized>(&'a Inner);

pub trait AsInvoker: RpcEndpoint {
    fn as_invoker<'a>(&'a self) -> Invoker<'a, Self>;
}

impl<Inner: RpcEndpoint> AsInvoker for Inner {
    fn as_invoker<'a>(&'a self) -> Invoker<'a, Self> {
        Invoker(self)
    }
}

impl<'a, Inner: RpcEndpoint + ?Sized> Invoker<'a, Inner> {
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

#[macro_export]
macro_rules! rpc_interface {
    {
        trait $interface_name:ident {
            $(
                $(#[doc = $doc:expr])*
                #[id = $rpc_uri:expr]
                fn $it:tt $args:tt -> Result<$ret:ty>;
            )*
        }
    }

     => {
        pub struct $interface_name<'a, Inner: $crate::rpc::wamp::RpcEndpoint + ?Sized>($crate::rpc::Invoker<'a, Inner>);

        impl<'a, Inner: $crate::rpc::wamp::RpcEndpoint + ?Sized + 'static> $interface_name<'a, Inner> {
            $(

                impl_async_rpc_item! {
                    $(#[doc = $doc])*
                    #[id = $rpc_uri]
                    fn $it $args -> Result<$ret, Error>;
                }

            )*
        }

     };
}

#[macro_export]
macro_rules! impl_async_rpc_item {
    {
                $(#[doc = $doc:expr])*
                #[id = $rpc_uri:expr]
                fn $name:ident(&self, $($arg_id:ident : $t:ty),*) -> Result<$ret:ty, Error>;

    }=> {
                $(#[doc = $doc])*
                #[doc = "RPC uri="]
                #[doc = $rpc_uri]
                pub fn $name(&self, $($arg_id : $t,)*) -> impl $crate::rpc::wamp::Future<Item=$ret, Error=$crate::rpc::wamp::Error> {
                    self.0.rpc_call($rpc_uri, &($($arg_id,)*))
                }
    };
    {
                $(#[doc = $doc:expr])*
                #[id = $rpc_uri:expr]
                fn $name:ident(&self) -> Result<$ret:ty, Error>;

    } => {
                $(#[doc = $doc])*
                #[doc = "RPC uri="]
                #[doc = $rpc_uri]
                pub fn $name<'b>(&'b self) -> impl $crate::rpc::wamp::Future<Item=$ret, Error=$crate::rpc::wamp::Error> + 'static {
                    self.0.rpc_call($rpc_uri, &())
                }
    };
}

#[cfg(test)]
#[allow(dead_code)]
mod test {

    use super::*;

    rpc_interface! {
    trait Test {

        /// Test function example
        #[id = "test"]
        fn test(&self, a : u8) -> Result<()>;

        #[id = "rpc.test.x2"]
        fn test2(&self) -> Result<Vec<String>>;
    }
    }

    pub trait AsTest: RpcEndpoint {
        fn as_test<'a>(&'a self) -> Test<'a, Self>;
    }

    impl<Endpoint: RpcEndpoint> AsTest for Endpoint {
        fn as_test<'a>(&'a self) -> Test<'a, Endpoint> {
            Test(self.as_invoker())
        }
    }

    struct RpcMock;

    impl RpcEndpoint for RpcMock {
        type Response = future::FutureResult<RpcCallResponse, Error>;

        fn rpc_call(&self, request: RpcCallRequest) -> Self::Response {
            eprintln!("request={:?}", request);
            future::ok(RpcCallResponse::default())
        }
    }

    #[test]
    fn test_compile() {
        let rpc = RpcMock;
        let t = rpc.as_test();

        let _ = t.test(10);
    }
}
