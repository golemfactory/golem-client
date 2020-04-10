use failure::Fallible;
use futures::{future, prelude::*};
use golem_rpc_api::net::{AsGolemNet, NodeInfo};
use std::str::FromStr;

///
/// Useful operation for building cli interface.
///
/// ## GolemId expansion
///
/// Allow user to provide shortcut to full public key.
/// For example `8c6c3bcb42a1a7a5...4acbcf47a0cb9361` instead of full
/// `8c6c3bcb42a1a7a555118a14c737e06aac16e17045523cef7c61facd17a8eeff971f377518773848312f312aaf1a2d8b3709376106cb3f6e4acbcf47a0cb9361`.
///
/// Format:
///
/// * <prefix>.... for example:  8c6c3bcb42a1a7a55511...
/// * ...<suffix> for example: ...3f6e4acbcf47a0cb9361
/// * <prefix>...<suffix> for example: 8c6c3bcb42a1...a0cb9361
///
/// Rules:
///
/// * expression must be specific enough it should have 80 bits specified
/// * expression should match to only 1 key on list.
///

#[derive(Debug, Clone)]
pub enum GolemIdPattern {
    Exact(String),
    MatchRule { prefix: Vec<u8>, suffix: Vec<u8> },
}

impl GolemIdPattern {
    #[inline]
    pub fn is_exact(&self) -> bool {
        match self {
            GolemIdPattern::Exact(_) => true,
            GolemIdPattern::MatchRule { .. } => false,
        }
    }

    #[inline]
    pub fn exact_value(&self) -> Option<&str> {
        match self {
            GolemIdPattern::Exact(v) => Some(v.as_ref()),
            GolemIdPattern::MatchRule { .. } => None,
        }
    }

    #[allow(unused)]
    pub fn match_to(&self, key: &str) -> Result<bool, failure::Error> {
        match self {
            GolemIdPattern::Exact(pat_key) => Ok(key == pat_key),
            GolemIdPattern::MatchRule { prefix, suffix } => {
                if key.len() != 128 {
                    Err(failure::format_err!("invalid key: {}", key))
                } else {
                    let key = key.as_bytes();
                    Ok(&key[..prefix.len()] == prefix.as_slice()
                        && &key[(128 - suffix.len())..] == suffix.as_slice())
                }
            }
        }
    }

    pub fn resolve<'a>(
        &self,
        key: impl Iterator<Item = &'a str>,
    ) -> Result<String, failure::Error> {
        match self {
            GolemIdPattern::Exact(pat_key) => Ok(pat_key.to_owned()),
            GolemIdPattern::MatchRule { prefix, suffix } => {
                let mut it = key
                    .filter(|key| {
                        if key.len() == 128 {
                            let key = key.as_bytes();
                            &key[..prefix.len()] == prefix.as_slice()
                                && &key[(128 - suffix.len())..] == suffix.as_slice()
                        } else {
                            false
                        }
                    })
                    .fuse();

                match (it.next(), it.next()) {
                    (Some(key), None) => Ok(key.to_owned()),
                    (None, None) => Err(failure::err_msg("key not found")),
                    _ => Err(failure::err_msg("key pattern is not selective enough")),
                }
            }
        }
    }
}

impl FromStr for GolemIdPattern {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, failure::Error> {
        if let Some(pos) = s.find("...") {
            let prefix = s[..pos].as_bytes();
            let suffix = s[pos + 3..].as_bytes();
            if prefix.len() + suffix.len() < 10 {
                return Err(failure::format_err!("pattern is not specific enough"));
            }
            if prefix.len() + suffix.len() > 128 {
                return Err(failure::format_err!("pattern has more than 512 bits"));
            }
            Ok(GolemIdPattern::MatchRule {
                prefix: prefix.into(),
                suffix: suffix.into(),
            })
        } else {
            if s.len() < 128 {
                return Err(failure::format_err!("value too short"));
            } else if s.len() > 128 {
                return Err(failure::format_err!("value too long"));
            }
            Ok(GolemIdPattern::Exact(s.to_owned()))
        }
    }
}

pub fn resolve_from_list(
    candidates: Vec<String>,
    patterns: Vec<GolemIdPattern>,
) -> Fallible<Vec<String>> {
    if patterns.iter().all(|p| p.is_exact()) {
        Ok(patterns
            .into_iter()
            .filter_map(|p| p.exact_value().map(|v| v.to_owned()))
            .collect())
    } else {
        patterns
            .into_iter()
            .map(|p| {
                p.resolve(candidates.iter().map(|c| c.as_ref()))
                    .map(|k| k.to_owned())
            })
            .collect()
    }
}

pub async fn resolve_from_known_hosts(
    endpoint: impl actix_wamp::RpcEndpoint + 'static,
    patterns: Vec<GolemIdPattern>,
) -> failure::Fallible<Vec<String>> {
    if patterns.iter().all(|p| p.is_exact()) {
        Ok(patterns
            .into_iter()
            .filter_map(|p| p.exact_value().map(|v| v.to_owned()))
            .collect())
    } else {
        let known_peers: Vec<NodeInfo> = endpoint.as_golem_net().get_known_peers().await?;

        let v: Result<Vec<String>, _> = patterns
            .into_iter()
            .map(|p| {
                p.resolve(known_peers.iter().map(|p| p.key.as_ref()))
                    .map(|k| k.to_owned())
            })
            .collect();
        v
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_pattern() {
        let key = "8c6c3bcb42a1a7a555118a14c737e06aac16e17045523cef7c61facd17a8eeff971f377518773848312f312aaf1a2d8b3709376106cb3f6e4acbcf47a0cb9361";
        let p: GolemIdPattern = key.parse().unwrap();
        let p1: GolemIdPattern = "8c6c3bcb42a1a7a55511...".parse().unwrap();
        let p2: GolemIdPattern = "...3f6e4acbcf47a0cb9361".parse().unwrap();
        let p3: GolemIdPattern = "8c6c3bcb42a1...a0cb9361".parse().unwrap();

        eprintln!("p={:?}, p1={:?} p2={:?} p3={:?}", p, p1, p2, p3);

        assert!(p.match_to(key).unwrap());
        assert!(p1.match_to(key).unwrap());
        assert!(p2.match_to(key).unwrap());
        assert!(p3.match_to(key).unwrap());
    }
}
