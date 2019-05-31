use futures::{future, prelude::*};
use std::fmt;
use std::path::{Path};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub enum Net {
    TestNet,
    MainNet,
}

impl Net {
    fn data_dir(&self) -> &str {
        match self {
            Net::MainNet => "mainnet",
            Net::TestNet => "rinkeby",
        }
    }
}

impl FromStr for Net {
    type Err = super::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mainnet" => Ok(Net::MainNet),
            "testnet" => Ok(Net::TestNet),
            _ => Err(super::Error::Other(format!("invalid net id: {}", s))),
        }
    }
}

impl fmt::Display for Net {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.data_dir())
    }
}

fn cert_hash(data_dir: impl AsRef<Path>, net: &Net) -> Result<Vec<u8>, super::Error> {
    let main_net_cert_path = data_dir
        .as_ref()
        .join(net.data_dir())
        .join("crossbar")
        .join("rpc_cert.pem");

    let cert = openssl::x509::X509::from_pem(std::fs::read(main_net_cert_path)?.as_ref())?;
    let bytes = cert.digest(openssl::hash::MessageDigest::sha1())?.to_vec();

    Ok(bytes)
}

fn hash_to_net(data_dir: &Path, hash: Vec<u8>) -> Option<Net> {
    [Net::MainNet, Net::TestNet]
        .iter()
        .cloned()
        .find(|net| match cert_hash(data_dir, net) {
            Ok(v) => v == hash,
            Err(e) => {
                log::warn!("unable to load cert from: {}, reason: {}", net, e);
                false
            }
        })
}

///
/// Connects to golemapp
///
/// ## Parameters
///
/// * `data_dir` - aplication datadir
/// * `net` - configuration type (mainnet/testnet) None for autodetect
/// * `rpc_addr` - force other than default rpc_address
///

pub fn connect_to_app(
    data_dir: &Path,
    net: impl Into<Option<Net>>,
    rpc_addr: Option<(&str, u16)>,
) -> impl Future<Item = impl actix_wamp::RpcEndpoint + Clone, Error = super::Error> {
    let (address, port) = rpc_addr.unwrap_or_else(|| ("127.0.0.1", 61000));
    let data_dir = data_dir.to_owned();
    actix_wamp::wss(address, port)
        .map_err(|e| super::Error::Other(format!("{}", e)))
        .and_then(move |(transport, hash)| {
            let net = match net.into().or_else(|| hash_to_net(&data_dir, hash?)) {
                Some(net) => net,
                None => {
                    return future::Either::B(future::err(super::Error::Other(
                        "invalid rpc cert".into(),
                    )))
                }
            };
            let net_data_dir = data_dir.join(net.data_dir());
            let auth_method =
                actix_wamp::challenge_response_auth(move |auth_id| -> Result<_, std::io::Error> {
                    let secret_file_path =
                        net_data_dir.join(format!("crossbar/secrets/{}.tck", auth_id));
                    log::debug!("reading secret from: {}", secret_file_path.display());
                    Ok(std::fs::read(secret_file_path)?)
                });
            future::Either::A(
                actix_wamp::SessionBuilder::with_auth("golem", "golemcli", auth_method)
                    .create(transport)
                    .from_err(),
            )
        })
}
