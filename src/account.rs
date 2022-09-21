use crate::transaction::{DisputedState, Transaction, TransactionRecord};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub struct Account {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

impl Account {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        }
    }

    pub fn process(
        &mut self,
        transaction: &Transaction,
        transactions: &mut HashMap<u32, TransactionRecord>,
    ) -> Result<(), ()> {
        use Transaction::*;

        if let Deposit { client, tx, amount } = *transaction {
            transactions.insert(
                tx,
                TransactionRecord {
                    amount,
                    client,
                    disputed: DisputedState::Undisputed,
                },
            );
        }

        match *transaction {
            Deposit { amount, .. } => self.deposit(amount)?,
            Withdrawal { amount, .. } => self.withdrawal(amount)?,
            Dispute { client, tx } => {
                let dependent_transaction = transactions.get_mut(&tx).ok_or(())?;

                if dependent_transaction.disputed != DisputedState::Undisputed
                    || dependent_transaction.client != client
                {
                    return Err(());
                }

                self.dispute(dependent_transaction.amount);
                dependent_transaction.disputed = DisputedState::Disputed;
            }
            Resolve { .. } => {
                let dependent_transaction =
                    dependent_transaction(transaction, transactions).ok_or(())?;
                self.resolve(dependent_transaction.amount);
                dependent_transaction.disputed = DisputedState::Resolved;
            }
            Chargeback { .. } => {
                let dependent_transaction =
                    dependent_transaction(transaction, transactions).ok_or(())?;
                self.chargeback(dependent_transaction.amount);
                dependent_transaction.disputed = DisputedState::Chargebacked;
            }
        }

        Ok(())
    }

    fn deposit(&mut self, amount: Decimal) -> Result<(), ()> {
        if amount >= Decimal::ZERO {
            self.available += amount;
            Ok(())
        } else {
            Err(())
        }
    }

    fn withdrawal(&mut self, amount: Decimal) -> Result<(), ()> {
        if amount >= Decimal::ZERO && self.available >= amount {
            self.available -= amount;
            Ok(())
        } else {
            Err(())
        }
    }

    fn dispute(&mut self, amount: Decimal) {
        // withdrawals fail if there are insufficient available funds I've not done that here
        // I'm not sure that clients should be able to have negative available balances
        // but I'm not sure I should fail the dispute if the client lacks available funds.
        self.available -= amount;
        self.held += amount;
    }

    fn resolve(&mut self, amount: Decimal) {
        self.held -= amount;
        self.available += amount;
    }

    fn chargeback(&mut self, amount: Decimal) {
        self.held -= amount;
        self.locked = true;
    }
}

/// Gets a transaction, ensures transaction is disputed and clients match
fn dependent_transaction<'a>(
    transaction: &Transaction,
    transactions: &'a mut HashMap<u32, TransactionRecord>,
) -> Option<&'a mut TransactionRecord> {
    if let Some(existing_transaction) = transactions.get_mut(&transaction.tx()) {
        let transaction_is_disputed = existing_transaction.disputed == DisputedState::Disputed;

        let matching_clients = transaction.client() == existing_transaction.client;

        if transaction_is_disputed && matching_clients {
            return Some(existing_transaction);
        }
    }

    None
}
