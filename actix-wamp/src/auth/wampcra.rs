//! WAMP Challenge-Response ("WAMP-CRA") authentication implementation

use super::{AuthMethod, Dict};
use crate::error::Error;
use std::marker::PhantomData;

const CRA_AUTH_METHOD_ID: &str = "wampcra";

struct WAMPCra<F, E>(F, PhantomData<E>)
where
    F: FnMut(&str) -> Result<Vec<u8>, E>,
    E: std::error::Error + Sync + Send + 'static;

impl<F, E> AuthMethod for WAMPCra<F, E>
where
    F: FnMut(&str) -> Result<Vec<u8>, E>,
    E: std::error::Error + Sync + Send + 'static,
{
    fn auth_method(&self) -> &str {
        CRA_AUTH_METHOD_ID
    }

    fn challenge(&mut self, auth_id: &str, extra: &Dict) -> Result<(String, Dict), Error> {
        use hmac::Mac;

        let challenge = match extra
            .get("challenge")
            .and_then(|challenge| challenge.as_str())
        {
            Some(challenge) => challenge,
            None => return Err(Error::protocol_err("missing challenge field")),
        };

        let secret = self.0(auth_id)?;
        let mut hmac = hmac::Hmac::<sha2::Sha256>::new_varkey(secret.as_ref())?;
        hmac.input(challenge.as_bytes());
        let r = hmac.result().code();
        Ok((base64::encode(&r), Dict::default()))
    }
}

/// Creates WAMP Challenge-Response authentication proivder from function providing shared secret.
///
pub fn challenge_response_auth<SecretProvider, Err>(
    secret_provider: SecretProvider,
) -> impl AuthMethod + Sync + Send + 'static
where
    SecretProvider: FnMut(&str) -> Result<Vec<u8>, Err> + Sync + Send + 'static,
    Err: std::error::Error + Sync + Send + 'static,
{
    WAMPCra(secret_provider, PhantomData)
}
