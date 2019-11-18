use bigdecimal::BigDecimal;
use ethkey::{Address, PublicKey};
use golem_rpc_api::pay::WalletOperationCurrency;
use num_bigint::{BigInt, ToBigInt};
use rustc_hex::FromHex;
use serde::Serialize;
use structopt::clap::arg_enum;

arg_enum! {
    #[derive(Debug, Serialize, Clone, Copy)]
    pub enum Currency {
        ETH,
        GNT
    }
}

arg_enum! {
    #[derive(Debug, Clone, Copy)]
    pub enum PaymentStatus {
        Awaiting,
        Confirmed
    }
}

impl PaymentStatus {
    pub fn is_match_with(&self, status: &golem_rpc_api::pay::PaymentStatus) -> bool {
        use golem_rpc_api::pay::PaymentStatus as RpcPaymentStatus;

        match self {
            PaymentStatus::Awaiting => match status {
                RpcPaymentStatus::Awaiting | RpcPaymentStatus::Sent => true,
                _ => false,
            },
            PaymentStatus::Confirmed => match status {
                RpcPaymentStatus::Confirmed => true,
                _ => false,
            },
        }
    }
}

#[inline]
fn eth_denoms() -> BigDecimal {
    BigDecimal::from(1000000000000000000u64)
}

impl Currency {
    pub fn format_decimal(&self, val: &bigdecimal::BigDecimal) -> String {
        format!("{} {}", val / eth_denoms(), self.as_str())
    }

    pub fn from_user(&self, val: &bigdecimal::BigDecimal) -> String {
        format!("{}", (val * eth_denoms()).to_bigint().unwrap())
    }

    pub fn as_str(&self) -> &str {
        match self {
            Currency::GNT => "GNT",
            Currency::ETH => "ETH",
        }
    }
}

impl From<WalletOperationCurrency> for Currency {
    fn from(currency: WalletOperationCurrency) -> Self {
        match currency {
            WalletOperationCurrency::GNT => Currency::GNT,
            WalletOperationCurrency::ETH => Currency::ETH,
        }
    }
}

pub fn public_to_addres(pubkey_hex: String) -> String {
    let pubkey_bytes: Vec<u8> = pubkey_hex.from_hex().unwrap();
    let pubkey = PublicKey::from_slice(&pubkey_bytes).unwrap();
    format!("{}", Address::from(pubkey.address().as_ref()))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_format() {
        let p = "19396108000000000".parse().unwrap();
        let p2 = "1899999999999999999900".parse().unwrap();
        assert_eq!("0.019396108 ETH", Currency::ETH.format_decimal(&p));
        assert_eq!(
            "1899.9999999999999999 GNT",
            Currency::GNT.format_decimal(&p2)
        );
    }

    #[test]
    fn test_public_to_addres() {
        let public = "782cc7dd72426893ae0d71477e41c41b03249a2b72e78eefcfe0baa9df604a8f979ab94cd23d872dac7bfa8d07d8b76b26efcbede7079f1c5cacd88fe9858f6e".into();
        let address = public_to_addres(public);
        assert_eq!("0x005b3bcf82085eededd551f50de7892471ffb272", address);
    }
}
