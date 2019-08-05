use failure::Error;
use futures::future;
use futures::Future;
use golem_rpc_api::core::AsGolemCore;
use zxcvbn::zxcvbn;

macro_rules! async_try {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => return future::Either::B(future::err(e.into())),
        }
    };
}

pub fn account_unlock(
    rpc: impl actix_wamp::RpcEndpoint + 'static,
) -> impl Future<Item = (), Error = Error> {
    rpc.as_golem().key_exists().from_err().and_then(move |key_exists| {
        if key_exists {
            eprintln!("Unlock your account to start golem");
        }
        else {
            eprintln!("No account found, generate one by setting a password");
        }
        let password = async_try!(rpassword::read_password_from_tty(Some("Password: ")));

        if password.is_empty() {
            return future::Either::B(future::err(failure::err_msg("No password provided")))
        }
        if !key_exists {

            if password.len() < 5 {
                return future::Either::B(future::err(failure::err_msg("Password is too short, minimum is 5")))
            }

            let estimate = async_try!(zxcvbn(&password, &["golem"]));

            if estimate.score < 2 {
                return future::Either::B(future::err(failure::err_msg("Password is not strong enough. Please use capitals, numbers and special characters.")))
            }
            let password2 = async_try!(rpassword::read_password_from_tty(Some(
                "Confirm password: ",
            )));

            if password2 != password {
                return future::Either::B(future::err(failure::err_msg("Password and confirmation do not match.")))
            }
        }

        future::Either::A(rpc.as_golem().set_password(password).from_err().and_then(|r| {
            if r {
                Ok(())
            }
            else {
                Err(failure::err_msg("invalid password"))
            }
        }))
    })
}
