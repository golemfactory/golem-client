use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::pay::{
    AsGolemPay, WalletOperation, WalletOperationCurrency, WalletOperationDirection,
    WalletOperationType,
};
use structopt::StructOpt;

const WALLET_COLUMNS: &[&str] = &["type", "status", "amount", "fee (ETH)", "task_id"];

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Show payments
    #[structopt(name = "show")]
    Show {
        #[structopt(long = "operation-type")]
        #[structopt(
            parse(try_from_str),
            raw(
                possible_values = "WalletOperationType::variants()",
                case_insensitive = "true"
            )
        )]
        /// Operation type
        operation_type: Option<WalletOperationType>,
        #[structopt(long)]
        #[structopt(
            parse(try_from_str),
            raw(
                possible_values = "WalletOperationDirection::variants()",
                case_insensitive = "true"
            )
        )]
        /// Operation direction
        direction: Option<WalletOperationDirection>,
        #[structopt(long = "per-page")]
        /// How many records per page
        per_page: Option<usize>,
        page: Option<usize>,
    },
}

impl Section {
    pub async fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Fallible<CommandResponse> {
        match self {
            Section::Show {
                operation_type,
                direction,
                per_page,
                page,
            } => {
                self.show(endpoint, *page, *per_page, operation_type, direction)
                    .await
            }
        }
    }

    async fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        page: Option<usize>,
        per_page: Option<usize>,
        operation_type: &Option<WalletOperationType>,
        direction: &Option<WalletOperationDirection>,
    ) -> Fallible<CommandResponse> {
        let (_total, operations) = endpoint
            .as_golem_pay()
            .get_operations(
                operation_type.clone(),
                direction.clone(),
                page.unwrap_or(1),
                per_page.unwrap_or(20),
            )
            .await?;

        let columns = WALLET_COLUMNS.iter().map(|&name| name.into()).collect();
        let values = operations
            .into_iter()
            .map(|operation: WalletOperation| {
                let operation_type = operation.operation_type;
                let status = operation.status;
                let amount = crate::eth::Currency::from(operation.currency)
                    .format_decimal(&operation.amount);

                let amount_str = match operation.direction {
                    WalletOperationDirection::Incoming => format!("+{}", amount),
                    WalletOperationDirection::Outgoing => format!("-{}", amount),
                };
                let fee = operation
                    .gas_cost
                    .map(|gas_cost| crate::eth::Currency::ETH.format_decimal(&gas_cost));

                let task_id = if let Some(task_payment) = operation.task_payment {
                    task_payment.task_id
                } else {
                    "".into()
                };
                serde_json::json!([operation_type, status, amount_str, fee, task_id])
            })
            .collect();
        Ok(ResponseTable { columns, values }.into())
    }
}
