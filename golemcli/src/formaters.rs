use golem_rpc_api::pay::DepositBalance;

pub trait Humanize {
    type Output;

    fn humanize(self) -> Self::Output;

}

impl Humanize for DepositBalance {
    type Output = serde_json::Value;

    fn humanize(self) -> Self::Output {
        serde_json::json!({
            "status": self.status,
            "timelock": self.timelock,
            "balance": crate::eth::Currency::GNT.format_decimal(&self.balance)
        })
    }
}