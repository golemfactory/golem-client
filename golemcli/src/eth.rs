use bigdecimal::BigDecimal;
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
    #[derive(Debug)]
    pub enum PaymentStatus {
        Awaiting,
        Confirmed
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

    pub fn as_str(&self) -> &str {
        match self {
            Currency::GNT => "GNT",
            Currency::ETH => "ETH",
        }
    }
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

}
