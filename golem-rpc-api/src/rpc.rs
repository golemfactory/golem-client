use actix_wamp::{Error, RpcCallRequest, RpcCallResponse, RpcEndpoint, ToArgs};
pub use futures::prelude::*;
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
    ) -> impl Future<Output = Result<Ret, Error>> + 'static {
        let request = match RpcCallRequest::with_args(uri, args) {
            Ok(resuest) => resuest,
            Err(e) => return future::Either::Right(future::err(e)),
        };
        future::Either::Left(self.0.rpc_call(request).and_then(
            move |RpcCallResponse { args, .. }| async move {
                if args.len() != 1 {
                    Err(Error::protocol_err(
                        "invalid rpc response, exactly 1 argument expected",
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
    ) -> impl Future<Output = Result<Ret, Error>> + 'static {
        let uri = uri.into().to_owned();
        let request = match RpcCallRequest::with_va_args(uri.clone(), va_args) {
            Ok(request) => request,
            Err(e) => return future::Either::Right(future::err(e)),
        };
        future::Either::Left(self.0.rpc_call(request).and_then(
            move |RpcCallResponse { args, .. }| async move {
                if args.len() != 1 {
                    Err(Error::protocol_err(
                        "invalid rpc response, exactly 1 argument expected",
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
                $(#[deprecated(since= $since:expr, note=$note:expr)])*
                #[rpc_uri = $rpc_uri:expr]
                fn $it:tt $args:tt -> Result<$ret:ty>;
            )*
        }

        $(
            converter $converter_name:ident $converter_method:ident;
        )?
    }

     => {
        pub struct $interface_name<'a, Inner: $crate::rpc::wamp::RpcEndpoint + ?Sized>($crate::rpc::Invoker<'a, Inner>);

        impl<'a, Inner: $crate::rpc::wamp::RpcEndpoint + ?Sized + 'static> $interface_name<'a, Inner> {
            $(

                impl_async_rpc_item! {
                    $(#[doc = $doc])*
                    #[doc = "Calls `"]
                    #[doc = $rpc_uri]
                    #[doc = "` RPC URI."]
                    $(#[deprecated(since= $since, note=$note)])*
                    #[rpc_uri = $rpc_uri]
                    fn $it $args -> Result<$ret, Error>;
                }

            )*
        }

        $(
            pub trait $converter_name : $crate::rpc::wamp::RpcEndpoint {
                fn $converter_method<'a>(&'a self ) -> $interface_name<'a, Self>;
            }

            impl<Endpoint: $crate::rpc::wamp::RpcEndpoint> $converter_name for Endpoint {
                fn $converter_method<'a>(&'a self) -> $interface_name<'a, Endpoint> {
                    $interface_name(self.as_invoker())
                }
            }
        )?

     };
}

#[macro_export]
#[doc(hidden)]
macro_rules! impl_async_rpc_item {
    {
                $(#[doc = $doc:expr])*
                $(#[deprecated(since= $since:expr, note=$note:expr)])*
                #[rpc_uri = $rpc_uri:expr]
                fn $name:ident(&self) -> Result<$ret:ty, Error>;

    } => {
                $(#[doc = $doc])*
                $(#[deprecated(since= $since, note=$note)])*
                pub fn $name<'b>(&'b self) -> impl $crate::rpc::wamp::Future<Output=Result<$ret, $crate::rpc::wamp::Error>> + 'static {
                    self.0.rpc_call($rpc_uri, &())
                }
    };

    {
                $(#[doc = $doc:expr])*
                $(#[deprecated(since= $since:expr, note=$note:expr)])*
                #[rpc_uri = $rpc_uri:expr]
                fn $name:ident(&self $(, $arg_id:ident : $t:ty)* $(, #[kwarg] $kw_arg_id:ident : $kw_t:ty)*) -> Result<$ret:ty, Error>;

    }=> {
                $(#[doc = $doc])*
                $(#[deprecated(since= $since, note=$note)])*
                #[allow(unused)]
                pub fn $name(&self $(, $arg_id : $t)* $(, $kw_arg_id : $kw_t)*) -> impl $crate::rpc::wamp::Future<Output=Result<$ret, $crate::rpc::wamp::Error>> {
                    self.0.rpc_call($rpc_uri, &($($arg_id,)*))
                }
    };


    {
                $(#[doc = $doc:expr])*
                $(#[deprecated(since= $since:expr, note=$note:expr)])*
                #[rpc_uri = $rpc_uri:expr]
                fn $name:ident(&self $(, #[kwarg] $kw_arg_id:ident : $kw_t:ty)+) -> Result<$ret:ty, Error>;

    } => {
                $(#[doc = $doc])*
                $(#[deprecated(since= $since, note=$note)])*
                #[allow(unused)]
                pub fn $name<'b>(&'b self $(, $kw_arg_id : $kw_t)+) -> impl $crate::rpc::wamp::Future<Item=$ret, Error=$crate::rpc::wamp::Error> + 'static {
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
            #[rpc_uri = "test"]
            fn test(&self, a : u8) -> Result<()>;

            #[rpc_uri = "rpc.test.x2"]
            fn test2(&self) -> Result<Vec<String>>;
        }

        converter AsTest as_test;
    }

    struct RpcMock;

    impl RpcEndpoint for RpcMock {
        type Response = future::Ready<Result<RpcCallResponse, Error>>;

        fn rpc_call(&self, request: RpcCallRequest) -> Self::Response {
            eprintln!("request={:?}", request);
            future::ok(RpcCallResponse {
                args: vec![serde_json::json!(["foo"])],
                kw_args: None,
            })
        }
    }
}
