use failure::Fallible;
use futures::prelude::*;
use golem_rpc_api::core::AsGolemCore;
use zxcvbn::zxcvbn;

pub async fn account_unlock(rpc: impl actix_wamp::RpcEndpoint + 'static) -> Fallible<()> {
    let key_exists = rpc.as_golem().key_exists().await?;

    if key_exists {
        eprintln!("Unlock your account to start golem");
    } else {
        eprintln!("No account found, generate one by setting a password");
    }
    let password = rpassword::read_password_from_tty(Some("Password: "))?;

    if password.is_empty() {
        return Err(failure::err_msg("No password provided"));
    }
    if !key_exists {
        if password.len() < 5 {
            return Err(failure::err_msg("Password is too short, minimum is 5"));
        }

        let estimate = zxcvbn(&password, &["golem"])?;

        if estimate.score < 2 {
            return Err(failure::err_msg("Password is not strong enough. Please use capitals, numbers and special characters."));
        }
        let password2 = rpassword::read_password_from_tty(Some("Confirm password: "))?;

        if password2 != password {
            return Err(failure::err_msg("Password and confirmation do not match."));
        }
    }

    if rpc.as_golem().set_password(password).await? {
        Ok(())
    } else {
        Err(failure::err_msg("invalid password"))
    }
}
