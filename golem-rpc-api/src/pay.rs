use crate::rpc::*;
use crate::serde::ts_seconds;
use bigdecimal::BigDecimal;
use serde::*;

rpc_interface! {
    trait GolemPay {
        #[id = "pay.incomes"]
        fn get_incomes_list(&self) -> Result<Vec<Income>>;

        //
        #[id = "pay.payments"]
        fn get_payments_list(&self, #[kwarg] _num: Option<u32>, #[kwarg] _last_seconds: Option<u32>) -> Result<Vec<Payment>>;

        //
        // TODO: kwargs limit=1000, offset=0
        #[id = "pay.deposit_payments"]
        fn get_deposit_payments_list(&self, #[kwarg] _limit : Option<usize>, #[kwarg] _offset : Option<usize>) -> Result<Option<Vec<DepositPayment>>>;

        #[id = "pay.deposit_balance"]
        fn get_deposit_balance(&self) -> Result<DepositBalance>;

    }

    converter AsGolemPay as_golem_pay;
}

/// The status of a payment.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(u8)]
#[serde(rename_all = "lowercase")]
pub enum PaymentStatus {
    /// Created but not introduced to the payment network.
    Awaiting = 1,

    /// Sent to the payment network.
    Sent = 2,

    /// Confirmed on the payment network.
    Confirmed = 3,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Income {
    pub subtask: String,
    pub payer: String,
    pub value: BigDecimal,
    // status
    pub status: PaymentStatus,
    pub transaction: Option<String>,
    #[serde(with = "ts_seconds")]
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(with = "ts_seconds")]
    pub modified: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payment {
    pub value: BigDecimal,
    pub fee: Option<BigDecimal>,
    pub subtask: String,
    pub payee: String,
    pub status: PaymentStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositPayment {
    pub value: BigDecimal,
    pub status: PaymentStatus,
    pub fee: Option<BigDecimal>,
    pub transaction: String,
    pub created: chrono::DateTime<chrono::Utc>,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum DepositStatus {
    Locked,
    Unlocking,
    Unlocked,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositBalance {
    pub status: DepositStatus,
    pub timelock: BigDecimal,
    #[serde(rename = "value")]
    pub balance: BigDecimal,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_income() {
        let str = r#"{
            "subtask": "1e12a7e4-50a3-11e9-9521-1bac4bb5328e",
            "payer": "1bac4bb5328ec9fc70b336e7c720dbb0c9cd23b8782c8bfc66f2a946ab036b8b6b71775cb04ad08d606f652ba476d9692c77cbd135f534d68f54c12e3edc039a",
            "value": "33333333333333334",
            "status": "awaiting",
            "transaction": null,
            "created": 1553699819.286437,
            "modified": 1557242132.961853
        }"#;

        let income: Income = serde_json::from_str(str).unwrap();

        eprintln!("income = {:?}", income);
    }

}
