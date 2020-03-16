use crate::rpc::*;
use crate::serde::{opt_ts_seconds, ts_seconds};
use bigdecimal::BigDecimal;
use failure::_core::str::FromStr;
use serde::*;

rpc_interface! {
    trait GolemPay {
        #[rpc_uri = "pay.operations"]
        fn get_operations(&self, operation_type: Option<WalletOperationType>, direction: Option<WalletOperationDirection>, page: usize, per_page: usize) -> Result<(u32, Vec<WalletOperation>)>;

        //#[deprecated(since="0.2.2", note="please use `get_operations` instead")]
        #[rpc_uri = "pay.incomes"]
        fn get_incomes_list(&self) -> Result<Vec<Income>>;

        //#[deprecated(since="0.2.2", note="please use `get_operations` instead")]
        #[rpc_uri = "pay.payments"]
        fn get_payments_list(&self, #[kwarg] _num: Option<u32>, #[kwarg] _last_seconds: Option<u32>) -> Result<Vec<Payment>>;

        // TODO: kwargs limit=1000, offset=0
        //#[deprecated(since="0.2.2", note="please use `get_operations` instead")]
        #[rpc_uri = "pay.deposit_payments"]
        fn get_deposit_payments_list(&self, #[kwarg] _limit : Option<usize>, #[kwarg] _offset : Option<usize>) -> Result<Option<Vec<DepositPayment>>>;

        #[rpc_uri = "pay.deposit_balance"]
        fn get_deposit_balance(&self) -> Result<Option<DepositBalance>>;

        #[rpc_uri = "pay.balance"]
        fn get_pay_balance(&self) -> Result<Balance>;

        #[rpc_uri = "pay.ident"]
        fn get_pay_ident(&self) -> Result<String>;
    }

    converter AsGolemPay as_golem_pay;
}

/// The status of a payment.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Ord, PartialOrd, PartialEq, Eq)]
#[repr(u8)]
#[serde(rename_all = "lowercase")]
pub enum PaymentStatus {
    /// Created but not introduced to the payment network.
    Awaiting = 1,

    /// Sent to the payment network.
    Sent = 2,

    /// Confirmed on the payment network.
    Confirmed = 3,

    /// Not confirmed on the payment network, but expected to be.
    Overdue = 4,
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

#[derive(Deserialize, Serialize)]
pub struct Balance {
    #[serde(default)]
    pub eth: BigDecimal,
    #[serde(default)]
    pub eth_lock: BigDecimal,
    #[serde(default)]
    pub av_gnt: BigDecimal,
    #[serde(default)]
    pub gnt_lock: BigDecimal,
    #[serde(default)]
    pub gnt_nonconverted: BigDecimal,
}

#[derive(Serialize, Deserialize)]
pub struct TaskPayment {
    pub node: crate::net::NodeInfo,
    pub task_id: String,
    pub subtask_id: String,
    pub charged_from_deposit: Option<bool>,
    #[serde(with = "opt_ts_seconds")]
    pub accepted_ts: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(with = "opt_ts_seconds")]
    pub settled_ts: Option<chrono::DateTime<chrono::Utc>>,
    pub missing_amount: BigDecimal,
    #[serde(with = "ts_seconds")]
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(with = "ts_seconds")]
    pub modified: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WalletOperationDirection {
    Incoming,
    Outgoing,
}

impl FromStr for WalletOperationDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "incoming" => Ok(Self::Incoming),
            "outgoing" => Ok(Self::Outgoing),
            _ => Err(format!("invalid value {}", s)),
        }
    }
}

impl WalletOperationDirection {
    pub fn variants() -> &'static [&'static str] {
        &["incoming", "outgoing"]
    }
}

//arg_enum! {
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WalletOperationType {
    Transfer,
    DepositTransfer,
    TaskPayment,
    DepositPayment,
}
//}

impl FromStr for WalletOperationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "transfer" => Self::Transfer,
            "deposit_transfer" => Self::DepositTransfer,
            "task_payment" => Self::TaskPayment,
            "deposit_payment" => Self::DepositPayment,
            _ => return Err(format!("invalid value {}", s)),
        })
    }
}

impl WalletOperationType {
    pub fn variants() -> &'static [&'static str] {
        &[
            "transfer",
            "deposit_transfer",
            "task_payment",
            "deposit_payment",
        ]
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum WalletOperationCurrency {
    ETH,
    GNT,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletOperationStatus {
    Awaiting,
    Sent,
    Confirmed,
    Overdue,
    Failed,
    ArbitragedByConcent,
}

#[derive(Serialize, Deserialize)]
pub struct WalletOperation {
    pub task_payment: Option<TaskPayment>,
    pub transaction_hash: Option<String>,
    pub direction: WalletOperationDirection,
    pub operation_type: WalletOperationType,
    pub status: WalletOperationStatus,
    pub sender_address: String,
    pub recipient_address: String,
    pub amount: BigDecimal,
    pub currency: WalletOperationCurrency,
    pub gas_cost: Option<BigDecimal>,
    #[serde(with = "ts_seconds")]
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(with = "ts_seconds")]
    pub modified: chrono::DateTime<chrono::Utc>,
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

    #[test]
    fn test_parse_wallet() {
        let str = r#"[
            13045, [
                {
                "task_payment": {
                    "node": {"node_name": "R03 Laughing Octopus", "key": "b7da70a8bbb439e8f1cdf7494ce163294a884ccaf9f117023e3cad5e5d4cb60655aec697c859fdeb9880ef94e3705ee0a2dd44eba7d68cabcd90e7972b48d00f", "prv_port": 40201, "pub_port": 40201, "p2p_prv_port": 40200, "p2p_pub_port": 40200, "prv_addr": "10.30.8.60", "pub_addr": "5.226.70.4", "prv_addresses": ["10.30.8.60", "172.16.136.1", "172.16.80.1", "172.17.0.1"], "hyperdrive_prv_port": 3282, "hyperdrive_pub_port": 3282, "port_statuses": {"40200": "timeout", "40201": "timeout", "3282": "timeout"}, "nat_type": []}, 
                    "task_id": "", 
                    "subtask_id": "02b2e4de-4184-11e8-8132-b7da70a8bbb4", 
                    "charged_from_deposit": null, 
                    "accepted_ts": 1535984463, 
                    "settled_ts": null, 
                    "missing_amount": "0", 
                    "created": 1540406987.388191, 
                    "modified": 1573816279
                    }, 
                "transaction_hash": "0x2a2dc7ac2044fd33d00131d9b4171e3ca8768c7f8f16c9de23d6f4329c1aec05", 
                "direction": "incoming", 
                "operation_type": "task_payment", 
                "status": "confirmed", 
                "sender_address": "", 
                "recipient_address": "0x0x5C49Ed170D0860273b39CCa561758523148d2bAe", 
                "amount": "790277777777777778", 
                "currency": "GNT", 
                "gas_cost": "0", 
                "created": 1540406987.388191, 
                "modified": 1573816279}
                ]
            ]"#;
        let result: (i32, Vec<WalletOperation>) = serde_json::from_str(str).unwrap();
    }

    #[test]
    fn test_variants() {
        for v in WalletOperationType::variants() {
            let _: WalletOperationType = v.parse().unwrap();
        }
    }
}
