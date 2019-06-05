use serde_derive::*;
use serde_json::{Map, Value};
use std::collections::btree_map::BTreeMap;
use std::fmt::Display;

pub type RoleMap = BTreeMap<Role, RoleDesc>;

pub type Dict = Map<String, Value>;

#[allow(dead_code)]
pub mod types {
    pub const HELLO: u8 = 01;
    pub const WELCOME: u8 = 02;
    pub const ABORT: u8 = 03;
    pub const GOODBYE: u8 = 06;
    pub const CHALLENGE: u8 = 04;
    pub const AUTHENTICATE: u8 = 05;
    pub const ERROR: u8 = 08;
    pub const PUBLISH: u8 = 16;
    pub const PUBLISHED: u8 = 17;
    pub const SUBSCRIBE: u8 = 32;
    pub const SUBSCRIBED: u8 = 33;
    pub const UNSUBSCRIBE: u8 = 34;
    pub const UNSUBSCRIBED: u8 = 35;
    pub const EVENT: u8 = 36;
    pub const CALL: u8 = 48;
    pub const CANCEL: u8 = 49;
    pub const RESULT: u8 = 50;
    pub const INTERRUPT: u8 = 69;
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RoleDesc {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

#[derive(Serialize, Deserialize, Hash, PartialOrd, PartialEq, Eq, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Caller,
    Callee,
    Publisher,
    Dealer,
}

/*
 Sygnalizacja błędów.

 1. Bład ma słownik z polem message oraz kod błędu.
 2. Bład wraca w wyniku odwbrania jakieś komendy.
 3. Błędy krytyczne lecą jako  [ABORT, Details|dict, Reason|uri] np.  [3, {"message": "The realm does not exist."}  "wamp.error.no_such_realm"]

*/

#[derive(Debug, Clone)]
pub struct WampError {
    pub code: ErrorKind,
    pub message: String,
    pub extra: Dict,
}

impl Display for WampError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}: {}", self.code.uri(), self.message)
    }
}

impl WampError {
    pub fn new(uri: &str, args: &rmpv::Value, kwargs: &rmpv::Value) -> Self {
        let code = ErrorKind::from_uri(uri);
        let extra: Dict = kwargs
            .as_map()
            .map(|v| {
                v.into_iter()
                    .filter_map(|(k, v)| {
                        let key = match k {
                            rmpv::Value::String(key) => key.clone().into_str()?,
                            _ => return None,
                        };
                        let value = serde_json::to_value(v).ok()?;

                        Some((key, value))
                    })
                    .collect()
            })
            .unwrap_or_else(|| Dict::new());
        let message = extra
            .get("message")
            .and_then(|v| v.as_str())
            .or_else(|| args[0].as_str())
            .unwrap_or_else(|| code.uri())
            .to_string();

        WampError {
            code,
            message,
            extra,
        }
    }
}

macro_rules! error_kinds {
    ($(
        $(#[$outer:meta])*
        $error_code:ident : $uri:expr),+) => {

        /// WAMP pre-defined error URIs.
        ///
        /// WAMP peers MUST use only the defined error messages.
        ///
        #[derive(Debug, PartialEq, Clone)]
        pub enum ErrorKind {
            $(
                $(#[$outer])*
                #[doc= "\n\n**WAMP uri:**"]
                #[doc= $uri]
                $error_code
            ),+
            ,
            /// Any other error code.
            Other(String)
        }

        impl ErrorKind {

            pub fn uri(&self) -> &str {
                match self {
                    $(
                        ErrorKind::$error_code => $uri,
                    )+
                    ErrorKind::Other(code) => code
                }
            }

            pub fn from_uri(uri : &str) -> Self {
                match uri {
                    $(
                        $uri => ErrorKind::$error_code,
                    )+
                    uri => ErrorKind::Other(uri.to_string())
                }
            }

        }

    };
}

error_kinds! {
    /// Peer provided an incorrect URI for any URI-based attribute of WAMP message, such as realm,
    /// topic or procedure
    InvalidURI : "wamp.error.invalid_uri",
    /// A Dealer could not perform a call, since no procedure is currently registered under the
    /// given URI.
    NoSuchProcedure : "wamp.error.no_such_procedure",
    /// A procedure could not be registered, since a procedure with the given URI is already
    /// registered.
    ProcedureAlreadyExists : "wamp.error.procedure_already_exists",
    /// A Dealer could not perform an unregister, since the given registration is not active.
    NoSuchRegistration : "wamp.error.no_such_registration",

    /// A Broker could not perform an unsubscribe, since the given subscription is not active.
    NoSuchSubscription : "wamp.error.no_such_subscription",

    /// A call failed since the given argument types or values are not acceptable to the called
    /// procedure. In this case the Callee may throw this error. Alternatively a Router may throw
    /// this error if it performed
    /// payload validation of a call, call result, call error or publish, and the payload did not
    /// conform to the requirements.
    InvalidArgument : "wamp.error.invalid_argument",

    /// The Peer is shutting down completely - used as a GOODBYE (or ABORT) reason.
    SystemShutdown : "wamp.error.system_shutdown",

    /// The Peer want to leave the realm - used as a GOODBYE reason.
    CloseRealm : "wamp.error.close_realm",

    /// A Peer acknowledges ending of a session - used as a GOODBYE reply reason.
    GoodbyeAndOut : "wamp.error.goodbye_and_out",

    /// A join, call, register, publish or subscribe failed, since the Peer is not authorized to
    /// perform the operation.
    NotAuthorized : "wamp.error.not_authorized",

    /// A Dealer or Broker could not determine if the Peer is authorized to perform a join, call,
    /// register, publish or subscribe, since the authorization operation itself failed.
    /// E.g. a custom authorizer did run into an error.
    AuthorizationFailed : "wamp.error.authorization_failed",

    /// Peer wanted to join a non-existing realm (and the Router did not allow to auto-create
    /// the realm).
    NoSuchRealm : "wamp.error.no_such_realm",

    /// A Peer was to be authenticated under a Role that does not (or no longer) exists on
    /// the Router. For example, the Peer was successfully authenticated, but the Role configured
    /// does not exists - hence there is some misconfiguration in the Router.
    NoSuchRole : "wamp.error.no_such_role",

    /// uriDealer or Callee canceled a call previously issued
    Canceled : "wamp.error.canceled",

    /// A Peer requested an interaction with an option that was disallowed by the Router
    OptionNotAllowed : "wamp.error.option_not_allowed",

    /// A Dealer could not perform a call, since a procedure with the given URI is registered,
    /// but Callee Black- and Whitelisting and/or Caller Exclusion lead to the exclusion of (any)
    /// Callee providing the procedure.
    NoEligibleCallee : "wamp.error.no_eligible_callee",

    /// A Router encountered a network failure
    NetworkFailure : "wamp.error.network_failure"
}

impl serde::Serialize for ErrorKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.uri())
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct HelloSpec<'a> {
    pub roles: RoleMap,
    #[serde(rename = "authmethods")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub auth_methods: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authid: Option<&'a str>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_uri() {
        assert_eq!(ErrorKind::Canceled.uri(), "wamp.error.canceled");
        assert_eq!(
            ErrorKind::from_uri("wamp.error.network_failure"),
            ErrorKind::NetworkFailure
        );
        eprintln!(
            "json={}",
            serde_json::to_string_pretty(&ErrorKind::AuthorizationFailed).unwrap()
        );
    }

    #[test]
    fn test_hello() {
        let val = HelloSpec {
            roles: vec![(Role::Caller, RoleDesc::default())]
                .into_iter()
                .collect(),
            auth_methods: vec![],
            authid: None,
        };

        eprintln!("json={}", serde_json::to_value(&val).unwrap());
    }
}
