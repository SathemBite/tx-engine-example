

struct TxEngine {
    users: std::collections::HashMap<UserId, Balances>,
}

struct Balances {
    balance_by_currency: std::collections::HashMap<Currency, Decimal>,
}

impl TxEngine {
    fn new() -> Self {
        TxEngine {
            users: std::collections::HashMap::new(),
        }
    }

    #[cfg(test)]
    fn new_with_state(users: std::collections::HashMap<String, UserData>) -> Self {
        TxEngine { users }
    }

    fn process_transaction(&mut self, tx: Transaction) {

    }
}