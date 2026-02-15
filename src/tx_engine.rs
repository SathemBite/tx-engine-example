use std::collections::{HashMap, HashSet};

use crate::{
    domain::{
        errors::AppError,
        types::{Amount, ClientId, TransactionType, TxID},
    },
    io::input::Transaction,
};

pub struct TxEngine {
    users: std::collections::HashMap<ClientId, ClientData>,
    processed_tx_ids: HashSet<TxID>,
}

struct ClientData {
    balances: Balances,
    txs: HashMap<TxID, TransactionRecord>,
    disputed_txs: HashMap<TxID, Amount>,
    frozen: bool,
}

pub struct ClientSnapshot {
    pub client_id: ClientId,
    pub available: Amount,
    pub held: Amount,
    pub locked: bool,
}

impl ClientSnapshot {
    pub fn total(&self) -> Amount {
        self.available + self.held
    }
}

trait ClientOwned {
    fn client_id(&self) -> &ClientId;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TransactionRecord {
    Deposit {
        client: ClientId,
        tx_id: TxID,
        amount: Amount,
    },
    Withdrawal {
        client: ClientId,
        tx_id: TxID,
        amount: Amount,
    },
    Dispute {
        client: ClientId,
        disputed_tx_id: TxID,
    },
    Resolve {
        client: ClientId,
        disputed_tx_id: TxID,
    },
    Chargeback {
        client: ClientId,
        disputed_tx_id: TxID,
    },
}

impl ClientOwned for TransactionRecord {
    fn client_id(&self) -> &ClientId {
        match self {
            TransactionRecord::Deposit { client, .. } => client,
            TransactionRecord::Withdrawal { client, .. } => client,
            TransactionRecord::Dispute { client, .. } => client,
            TransactionRecord::Resolve { client, .. } => client,
            TransactionRecord::Chargeback { client, .. } => client,
        }
    }
}

impl ClientData {
    fn init() -> Self {
        ClientData {
            balances: Balances::init(),
            txs: HashMap::new(),
            disputed_txs: HashMap::new(),
            frozen: false,
        }
    }
}

struct Balances {
    available: Amount,
    held: Amount,
}

impl Balances {
    fn init() -> Self {
        Balances {
            available: Amount::ZERO,
            held: Amount::ZERO,
        }
    }
}

impl Default for TxEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TxEngine {
    pub fn new() -> Self {
        TxEngine {
            users: std::collections::HashMap::new(),
            processed_tx_ids: HashSet::new(),
        }
    }

    pub fn clients_snapshot(&self) -> Vec<ClientSnapshot> {
        let mut snapshots: Vec<ClientSnapshot> = self
            .users
            .iter()
            .map(|(client_id, data)| ClientSnapshot {
                client_id: *client_id,
                available: data.balances.available,
                held: data.balances.held,
                locked: data.frozen,
            })
            .collect();

        snapshots.sort_by_key(|snapshot| snapshot.client_id.0);
        snapshots
    }

    pub fn process_transaction(&mut self, tx: &Transaction) -> Result<(), AppError> {
        let record = Self::to_transaction_record(tx)?;
        self.process_transaction_internal(&record)?;
        self.record_processed_transaction(record);
        Ok(())
    }

    fn process_transaction_internal(&mut self, tx: &TransactionRecord) -> Result<(), AppError> {
        self.check_duplicate_tx(tx)?;
        self.check_frozen(tx.client_id())?;

        match tx {
            TransactionRecord::Deposit {
                client,
                tx_id: _,
                amount,
            } => {
                let user = self.users.entry(*client).or_insert_with(ClientData::init);
                user.balances.available += *amount;
            }

            TransactionRecord::Withdrawal {
                client,
                tx_id: _,
                amount,
            } => {
                let user = self.users.entry(*client).or_insert_with(ClientData::init);
                if (user.balances.available - *amount) < Amount::ZERO {
                    return Err(AppError::TxProcessingNonCritical(format!(
                        "Insufficient funds for user {}: available {}, attempted withdrawal {}",
                        client, user.balances.available, amount
                    )));
                }
                user.balances.available -= *amount;
            }

            TransactionRecord::Dispute {
                client,
                disputed_tx_id,
            } => {
                let user = match self.users.get_mut(client) {
                    Some(user) => user,
                    None => {
                        return Err(AppError::TxProcessingNonCritical(format!(
                            "Cannot dispute transaction {} for user {}, client not found",
                            disputed_tx_id, client
                        )));
                    }
                };

                if user.disputed_txs.contains_key(disputed_tx_id) {
                    return Err(AppError::TxProcessingNonCritical(format!(
                        "Transaction {} for user {} is already disputed",
                        disputed_tx_id, client
                    )));
                }

                let diputed_tx = match user.txs.get(disputed_tx_id) {
                    Some(tx) => tx,
                    None => {
                        return Err(AppError::TxProcessingNonCritical(format!(
                            "Disputed transaction {} not found for user {}",
                            disputed_tx_id, client
                        )));
                    }
                };

                let balance_diff = match diputed_tx {
                    TransactionRecord::Deposit { amount, .. } => *amount,

                    TransactionRecord::Withdrawal { .. }
                    | TransactionRecord::Dispute { .. }
                    | TransactionRecord::Resolve { .. }
                    | TransactionRecord::Chargeback { .. } => {
                        return Err(AppError::TxProcessingNonCritical(format!(
                            "Cannot dispute transaction {} for user {}, not a deposit",
                            disputed_tx_id, client
                        )));
                    }
                };

                user.balances.available -= balance_diff;
                user.balances.held += balance_diff;
                user.disputed_txs.insert(*disputed_tx_id, balance_diff);
            }

            TransactionRecord::Resolve {
                client,
                disputed_tx_id,
            } => {
                let user = match self.users.get_mut(client) {
                    Some(user) => user,
                    None => {
                        return Err(AppError::TxProcessingNonCritical(format!(
                            "Cannot resolve disputed transaction {} for user {}, client not found",
                            disputed_tx_id, client
                        )));
                    }
                };

                let disputed_tx_diff = match user.disputed_txs.get(disputed_tx_id) {
                    Some(amount) => amount,
                    None => {
                        return Err(AppError::TxProcessingNonCritical(format!(
                            "Cannot resolve disputed transaction {} for user {}, not in dispute",
                            disputed_tx_id, client
                        )));
                    }
                };

                user.balances.available += *disputed_tx_diff;
                user.balances.held -= *disputed_tx_diff;
                user.disputed_txs.remove(disputed_tx_id);
            }

            TransactionRecord::Chargeback {
                client,
                disputed_tx_id,
            } => {
                let user = match self.users.get_mut(client) {
                    Some(user) => user,
                    None => {
                        return Err(AppError::TxProcessingNonCritical(format!(
                            "Cannot chargeback disputed transaction {} for user {}, client not found",
                            disputed_tx_id, client
                        )));
                    }
                };

                let disputed_tx_diff = match user.disputed_txs.get(disputed_tx_id) {
                    Some(amount) => amount,
                    None => {
                        return Err(AppError::TxProcessingNonCritical(format!(
                            "Cannot chargeback disputed transaction {} for user {}, not in dispute",
                            disputed_tx_id, client
                        )));
                    }
                };

                user.balances.held -= *disputed_tx_diff;
                user.disputed_txs.remove(disputed_tx_id);
                user.frozen = true;
            }
        }

        Ok(())
    }

    fn check_duplicate_tx(&self, tx: &TransactionRecord) -> Result<(), AppError> {
        match tx {
            TransactionRecord::Deposit { tx_id, .. }
            | TransactionRecord::Withdrawal { tx_id, .. } => {
                if self.processed_tx_ids.contains(tx_id) {
                    return Err(AppError::TxProcessingNonCritical(format!(
                        "Duplicate transaction ID {}",
                        tx_id
                    )));
                }
                Ok(())
            }
            TransactionRecord::Dispute { .. }
            | TransactionRecord::Resolve { .. }
            | TransactionRecord::Chargeback { .. } => Ok(()),
        }
    }

    fn check_frozen(&self, client: &ClientId) -> Result<(), AppError> {
        if self.users.get(client).is_some_and(|user| user.frozen) {
            return Err(AppError::TxProcessingNonCritical(format!(
                "Account {} is frozen",
                client
            )));
        }
        Ok(())
    }

    fn to_transaction_record(tx: &Transaction) -> Result<TransactionRecord, AppError> {
        match tx.op_type {
            TransactionType::Deposit => {
                let amount = tx.amount.ok_or_else(|| {
                    AppError::TxProcessingNonCritical(format!(
                        "Missing amount for deposit tx {} and client {}",
                        tx.tx_id, tx.client
                    ))
                })?;
                Ok(TransactionRecord::Deposit {
                    client: tx.client,
                    tx_id: tx.tx_id,
                    amount,
                })
            }
            TransactionType::Withdrawal => {
                let amount = tx.amount.ok_or_else(|| {
                    AppError::TxProcessingNonCritical(format!(
                        "Missing amount for withdrawal tx {} and client {}",
                        tx.tx_id, tx.client
                    ))
                })?;
                Ok(TransactionRecord::Withdrawal {
                    client: tx.client,
                    tx_id: tx.tx_id,
                    amount,
                })
            }
            TransactionType::Dispute => Ok(TransactionRecord::Dispute {
                client: tx.client,
                disputed_tx_id: tx.tx_id,
            }),
            TransactionType::Resolve => Ok(TransactionRecord::Resolve {
                client: tx.client,
                disputed_tx_id: tx.tx_id,
            }),
            TransactionType::Chargeback => Ok(TransactionRecord::Chargeback {
                client: tx.client,
                disputed_tx_id: tx.tx_id,
            }),
        }
    }

    fn record_processed_transaction(&mut self, tx: TransactionRecord) {
        match tx {
            TransactionRecord::Deposit { client, tx_id, .. }
            | TransactionRecord::Withdrawal { client, tx_id, .. } => {
                self.processed_tx_ids.insert(tx_id);
                if let Some(user) = self.users.get_mut(&client) {
                    user.txs.insert(tx_id, tx);
                }
            }
            TransactionRecord::Dispute { .. }
            | TransactionRecord::Resolve { .. }
            | TransactionRecord::Chargeback { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_tx(
        op_type: TransactionType,
        client: u16,
        tx_id: u32,
        amount: Option<Amount>,
    ) -> Transaction {
        Transaction {
            op_type,
            client: ClientId(client),
            tx_id: TxID(tx_id),
            amount,
        }
    }

    fn snapshot_for(engine: &TxEngine, client_id: u16) -> ClientSnapshot {
        engine
            .clients_snapshot()
            .into_iter()
            .find(|snapshot| snapshot.client_id == ClientId(client_id))
            .expect("snapshot for client must exist")
    }

    #[test]
    fn deposit_increases_available_and_total() {
        let mut engine = TxEngine::new();
        let tx = make_tx(TransactionType::Deposit, 1, 1, Some(Amount::new(dec!(5.5))));

        engine.process_transaction(&tx).unwrap();

        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::new(dec!(5.5)));
        assert_eq!(snapshot.held, Amount::ZERO);
        assert_eq!(snapshot.total(), Amount::new(dec!(5.5)));
        assert!(!snapshot.locked);
    }

    #[test]
    fn withdrawal_with_insufficient_funds_is_rejected_without_state_change() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                1,
                Some(Amount::new(dec!(1.0))),
            ))
            .unwrap();

        let result = engine.process_transaction(&make_tx(
            TransactionType::Withdrawal,
            1,
            2,
            Some(Amount::new(dec!(2.0))),
        ));

        assert!(matches!(result, Err(AppError::TxProcessingNonCritical(_))));
        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::new(dec!(1.0)));
        assert_eq!(snapshot.held, Amount::ZERO);
        assert_eq!(snapshot.total(), Amount::new(dec!(1.0)));
    }

    #[test]
    fn withdrawal_successfully_reduces_available_and_total() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                1,
                Some(Amount::new(dec!(5.0))),
            ))
            .unwrap();

        engine
            .process_transaction(&make_tx(
                TransactionType::Withdrawal,
                1,
                2,
                Some(Amount::new(dec!(1.5))),
            ))
            .unwrap();

        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::new(dec!(3.5)));
        assert_eq!(snapshot.held, Amount::ZERO);
        assert_eq!(snapshot.total(), Amount::new(dec!(3.5)));
    }

    #[test]
    fn dispute_on_deposit_moves_funds_to_held_even_if_available_goes_negative() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                1,
                Some(Amount::new(dec!(2.0))),
            ))
            .unwrap();
        engine
            .process_transaction(&make_tx(
                TransactionType::Withdrawal,
                1,
                2,
                Some(Amount::new(dec!(1.5))),
            ))
            .unwrap();

        engine
            .process_transaction(&make_tx(TransactionType::Dispute, 1, 1, None))
            .unwrap();

        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::new(dec!(-1.5)));
        assert_eq!(snapshot.held, Amount::new(dec!(2.0)));
        assert_eq!(snapshot.total(), Amount::new(dec!(0.5)));
    }

    #[test]
    fn dispute_on_unknown_tx_for_existing_client_is_rejected() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                1,
                Some(Amount::new(dec!(2.0))),
            ))
            .unwrap();

        let result = engine.process_transaction(&make_tx(TransactionType::Dispute, 1, 99, None));

        assert!(matches!(result, Err(AppError::TxProcessingNonCritical(_))));
        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::new(dec!(2.0)));
        assert_eq!(snapshot.held, Amount::ZERO);
    }

    #[test]
    fn duplicate_dispute_on_same_tx_is_rejected() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                1,
                Some(Amount::new(dec!(2.0))),
            ))
            .unwrap();
        engine
            .process_transaction(&make_tx(TransactionType::Dispute, 1, 1, None))
            .unwrap();

        let result = engine.process_transaction(&make_tx(TransactionType::Dispute, 1, 1, None));

        assert!(matches!(result, Err(AppError::TxProcessingNonCritical(_))));
        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::new(dec!(0.0)));
        assert_eq!(snapshot.held, Amount::new(dec!(2.0)));
    }

    #[test]
    fn dispute_on_withdrawal_is_rejected() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                1,
                Some(Amount::new(dec!(5.0))),
            ))
            .unwrap();
        engine
            .process_transaction(&make_tx(
                TransactionType::Withdrawal,
                1,
                2,
                Some(Amount::new(dec!(1.0))),
            ))
            .unwrap();

        let result = engine.process_transaction(&make_tx(TransactionType::Dispute, 1, 2, None));

        assert!(matches!(result, Err(AppError::TxProcessingNonCritical(_))));
        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::new(dec!(4.0)));
        assert_eq!(snapshot.held, Amount::ZERO);
        assert_eq!(snapshot.total(), Amount::new(dec!(4.0)));
    }

    #[test]
    fn resolve_releases_held_funds() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                1,
                Some(Amount::new(dec!(3.0))),
            ))
            .unwrap();
        engine
            .process_transaction(&make_tx(TransactionType::Dispute, 1, 1, None))
            .unwrap();

        engine
            .process_transaction(&make_tx(TransactionType::Resolve, 1, 1, None))
            .unwrap();

        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::new(dec!(3.0)));
        assert_eq!(snapshot.held, Amount::ZERO);
        assert_eq!(snapshot.total(), Amount::new(dec!(3.0)));
    }

    #[test]
    fn resolve_without_active_dispute_is_rejected() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                1,
                Some(Amount::new(dec!(3.0))),
            ))
            .unwrap();

        let result = engine.process_transaction(&make_tx(TransactionType::Resolve, 1, 1, None));

        assert!(matches!(result, Err(AppError::TxProcessingNonCritical(_))));
        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::new(dec!(3.0)));
        assert_eq!(snapshot.held, Amount::ZERO);
    }

    #[test]
    fn chargeback_locks_account_and_future_transactions_are_rejected() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                1,
                Some(Amount::new(dec!(3.0))),
            ))
            .unwrap();
        engine
            .process_transaction(&make_tx(TransactionType::Dispute, 1, 1, None))
            .unwrap();
        engine
            .process_transaction(&make_tx(TransactionType::Chargeback, 1, 1, None))
            .unwrap();

        let post_chargeback_tx =
            make_tx(TransactionType::Deposit, 1, 2, Some(Amount::new(dec!(1.0))));
        let result = engine.process_transaction(&post_chargeback_tx);

        assert!(matches!(result, Err(AppError::TxProcessingNonCritical(_))));
        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::ZERO);
        assert_eq!(snapshot.held, Amount::ZERO);
        assert_eq!(snapshot.total(), Amount::ZERO);
        assert!(snapshot.locked);
    }

    #[test]
    fn chargeback_without_active_dispute_is_rejected() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                1,
                Some(Amount::new(dec!(3.0))),
            ))
            .unwrap();

        let result = engine.process_transaction(&make_tx(TransactionType::Chargeback, 1, 1, None));

        assert!(matches!(result, Err(AppError::TxProcessingNonCritical(_))));
        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::new(dec!(3.0)));
        assert_eq!(snapshot.held, Amount::ZERO);
        assert!(!snapshot.locked);
    }

    #[test]
    fn frozen_account_rejects_non_deposit_ops_too() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                1,
                Some(Amount::new(dec!(4.0))),
            ))
            .unwrap();
        engine
            .process_transaction(&make_tx(TransactionType::Dispute, 1, 1, None))
            .unwrap();
        engine
            .process_transaction(&make_tx(TransactionType::Chargeback, 1, 1, None))
            .unwrap();

        let resolve_result =
            engine.process_transaction(&make_tx(TransactionType::Resolve, 1, 1, None));
        let dispute_result =
            engine.process_transaction(&make_tx(TransactionType::Dispute, 1, 1, None));
        let chargeback_result =
            engine.process_transaction(&make_tx(TransactionType::Chargeback, 1, 1, None));

        assert!(matches!(
            resolve_result,
            Err(AppError::TxProcessingNonCritical(_))
        ));
        assert!(matches!(
            dispute_result,
            Err(AppError::TxProcessingNonCritical(_))
        ));
        assert!(matches!(
            chargeback_result,
            Err(AppError::TxProcessingNonCritical(_))
        ));
    }

    #[test]
    fn duplicate_tx_id_is_rejected_globally_across_clients() {
        let mut engine = TxEngine::new();
        engine
            .process_transaction(&make_tx(
                TransactionType::Deposit,
                1,
                10,
                Some(Amount::new(dec!(1.0))),
            ))
            .unwrap();

        let result = engine.process_transaction(&make_tx(
            TransactionType::Deposit,
            2,
            10,
            Some(Amount::new(dec!(2.0))),
        ));

        assert!(matches!(result, Err(AppError::TxProcessingNonCritical(_))));
        assert_eq!(engine.clients_snapshot().len(), 1);
        let snapshot = snapshot_for(&engine, 1);
        assert_eq!(snapshot.available, Amount::new(dec!(1.0)));
    }

    #[test]
    fn invalid_non_deposit_ops_for_unknown_client_do_not_create_state() {
        let mut engine = TxEngine::new();

        let dispute_result =
            engine.process_transaction(&make_tx(TransactionType::Dispute, 9, 1, None));
        let resolve_result =
            engine.process_transaction(&make_tx(TransactionType::Resolve, 9, 1, None));
        let chargeback_result =
            engine.process_transaction(&make_tx(TransactionType::Chargeback, 9, 1, None));

        assert!(matches!(
            dispute_result,
            Err(AppError::TxProcessingNonCritical(_))
        ));
        assert!(matches!(
            resolve_result,
            Err(AppError::TxProcessingNonCritical(_))
        ));
        assert!(matches!(
            chargeback_result,
            Err(AppError::TxProcessingNonCritical(_))
        ));
        assert!(engine.clients_snapshot().is_empty());
    }

    #[test]
    fn missing_amount_for_deposit_is_rejected() {
        let mut engine = TxEngine::new();
        let result = engine.process_transaction(&make_tx(TransactionType::Deposit, 1, 1, None));

        assert!(matches!(result, Err(AppError::TxProcessingNonCritical(_))));
        assert!(engine.clients_snapshot().is_empty());
    }

    #[test]
    fn missing_amount_for_withdrawal_is_rejected() {
        let mut engine = TxEngine::new();
        let result = engine.process_transaction(&make_tx(TransactionType::Withdrawal, 1, 1, None));

        assert!(matches!(result, Err(AppError::TxProcessingNonCritical(_))));
        assert!(engine.clients_snapshot().is_empty());
    }
}
