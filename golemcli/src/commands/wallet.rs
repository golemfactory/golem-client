use crate::context::*;
use futures::prelude::*;
use golem_rpc_api::pay::{AsGolemPay, WalletOperation, WalletOperationCurrency, WalletOperationDirection, WalletOperationType};
use structopt::StructOpt;

const WALLET_COLUMNS: &[&str] = &["type", "status", "amount", "fee (ETH)", "task_id"];

#[derive(StructOpt, Debug)]
pub enum Section {
    /// Show payments
    #[structopt(name = "show")]
    Show {
        #[structopt(long)]
        operation_type: Option<WalletOperationType>,
        #[structopt(long)]
        direction: Option<WalletOperationDirection>,
        #[structopt(long)]
        per_page: Option<usize>,
        page: Option<usize>,
    },
}

impl Section {
    pub fn run(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
    ) -> Box<dyn Future<Item = CommandResponse, Error = Error> + 'static> {
        match self {
            Section::Show {
                operation_type,
                direction,
                per_page,
                page,
            } => Box::new(self.show(endpoint, *page, *per_page, operation_type, direction)),
        }
    }

    pub fn show(
        &self,
        endpoint: impl actix_wamp::RpcEndpoint + Clone + 'static,
        page: Option<usize>,
        per_page: Option<usize>,
        operation_type: &Option<WalletOperationType>,
        direction: &Option<WalletOperationDirection>,
    ) -> impl Future<Item = CommandResponse, Error = Error> + 'static {

        endpoint
            .as_golem_pay()
            .get_operations(operation_type.clone(), direction.clone(), page.unwrap_or(1), per_page.unwrap_or(20))
            .from_err()
            .and_then(move |result: (u32, Vec<WalletOperation>)| {
                let (total, operations) = result;
                let columns = WALLET_COLUMNS.iter().map(|&name| name.into()).collect();
                let values = operations
                    .into_iter()
                    .map(|operation: WalletOperation| {
                        let type_ = operation.operation_type;
                        let status = operation.status;
                        let amount: String;
                        match operation.currency {
                            WalletOperationCurrency::GNT => amount = crate::eth::Currency::GNT.format_decimal(&operation.amount),
                            WalletOperationCurrency::ETH => amount = crate::eth::Currency::ETH.format_decimal(&operation.amount),
                        }
                        let amount_str: String;
                        match operation.direction {
                            WalletOperationDirection::Incoming => {
                                amount_str = format!("+{}", amount);
                            }
                            WalletOperationDirection::Outgoing => {
                                amount_str = format!("-{}", amount);
                            }
                        };
                        let fee = operation
                            .gas_cost
                            .map(|gas_cost| crate::eth::Currency::ETH.format_decimal(&gas_cost));

                        let task_id: String;
                        match operation.task_payment {
                            None => task_id = "".to_string(),
                            Some(task_payment) => task_id = task_payment.task_id,
                        };
                        serde_json::json!([type_, status, amount_str, fee, task_id])
                    })
                    .collect();
                Ok(ResponseTable { columns, values }.into())
            })
    }
}
