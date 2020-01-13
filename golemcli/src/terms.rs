use actix_wamp::RpcEndpoint;
use futures::Future;
use golem_rpc_api::terms::AsGolemTerms;
use promptly::{Promptable, Prompter};
use std::io;

pub enum TermsQuery {
    Show,
    Accept,
    Reject,
}

impl Promptable for TermsQuery {
    fn prompt<S: AsRef<str>>(msg: S) -> Self {
        Prompter::new().prompt_then(msg, |s| match &*s.to_lowercase() {
            "a" | "accept" => Ok(TermsQuery::Accept),
            "r" | "reject" => Ok(TermsQuery::Reject),
            "s" | "show" => Ok(TermsQuery::Show),
            v => Err(format!(
                "wrong value {}, it should be one of (a)ccept / (r)eject / (s)show",
                v
            )),
        })
    }

    fn prompt_opt<S: AsRef<str>>(msg: S) -> Option<Self> {
        Prompter::new().prompt_then(msg, |s| match &*s.to_lowercase() {
            "" => Ok(None),
            "a" | "accept" => Ok(Some(TermsQuery::Accept)),
            "r" | "reject" => Ok(Some(TermsQuery::Reject)),
            "s" | "show" => Ok(Some(TermsQuery::Show)),
            v => Err(format!(
                "wrong value {}, it should be one of (a)ccept / (r)eject / (s)show",
                v
            )),
        })
    }

    fn prompt_default<S: AsRef<str>>(msg: S, default: Self) -> Self {
        Self::prompt_opt(msg).unwrap_or(default)
    }
}

pub async fn get_terms_text(
    endpoint: &(impl RpcEndpoint + 'static),
) -> Result<String, actix_wamp::Error> {
    Ok(html2text::from_read(
        io::Cursor::new(endpoint.as_golem_terms().show_terms().await?),
        78,
    ))
}
