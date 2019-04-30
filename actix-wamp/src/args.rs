use crate::error::Error;
use crate::messages::Dict;
use actix::prelude::*;
use serde::Serialize;
use serde_json::Value;
use std::borrow::Cow;

pub trait ToArgs {
    fn as_json(&self) -> Result<Option<Value>, Error>;
}

impl ToArgs for () {
    fn as_json(&self) -> Result<Option<Value>, Error> {
        Ok(None)
    }
}

macro_rules! def_args {
    ($($t:ident),+) => {
        impl< $($t : Serialize),+ > ToArgs for ($($t,)+) {
            fn as_json(&self) -> Result<Option<Value>, Error> {
                Ok(Some(serde_json::to_value(self)?))
            }
        }
    };
}

def_args!(T1);
def_args!(T1, T2);
def_args!(T1, T2, T3);
def_args!(T1, T2, T3, T4);

#[derive(Debug)]
pub struct RpcCallRequest {
    pub(crate) uri: Cow<'static, str>,
    pub(crate) options: Option<Dict>,
    pub(crate) args: Option<Value>,
    pub(crate) kw_args: Option<Value>,
}

impl RpcCallRequest {
    pub fn with_va_args<T: Serialize>(
        uri: impl Into<Cow<'static, str>>,
        va_args: &Vec<T>,
    ) -> Result<Self, Error> {
        Ok(RpcCallRequest {
            uri: uri.into(),
            options: None,
            args: Some(serde_json::to_value(va_args)?),
            kw_args: None,
        })
    }

    pub fn with_no_args(uri: &'static str) -> Self {
        RpcCallRequest {
            uri: Cow::Borrowed(uri),
            options: None,
            args: None,
            kw_args: None,
        }
    }

    pub fn with_args(uri: &'static str, args: &impl crate::args::ToArgs) -> Result<Self, Error> {
        Ok(RpcCallRequest {
            uri: Cow::Borrowed(uri),
            options: None,
            args: args.as_json()?,
            kw_args: None,
        })
    }
}

#[derive(Debug)]
pub struct RpcCallResponse {
    pub args: Vec<Value>,
    pub kw_args: Option<Dict>,
}

impl Message for RpcCallRequest {
    type Result = Result<RpcCallResponse, Error>;
}

pub trait RpcEndpoint {
    type Response: Future<Item = RpcCallResponse, Error = Error> + 'static;

    fn rpc_call(&self, request: RpcCallRequest) -> Self::Response;
}
