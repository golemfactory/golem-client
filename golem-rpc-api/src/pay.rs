use super::Map;
use crate::rpc::*;
use bigdecimal::BigDecimal;
use serde_derive::*;
use serde_json::Value;

rpc_interface! {
    trait GolemPay {
        #[id = "pay.incomes"]
        fn get_incomes_list(&self) -> Result<Vec<Income>>;

        //
        #[id = "pay.payments"]
        fn get_payments_list(&self, num: Option<u32>, last_seconds: Option<u32>) -> Result<Vec<Payment>>;

        //
        // TODO: kwargs limit=1000, offset=0
        #[id = "pay.deposit_payments"]
        fn get_deposit_payments_list(&self) -> Result<Option<Vec<DepositPayment>>>;
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
    pub transaction: String,
    pub created: chrono::DateTime<chrono::Utc>,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
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
