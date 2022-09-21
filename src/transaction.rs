use crate::account::Account;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::{collections::HashMap, fs::File, path::Path};

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Type {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Deserialize, Debug, Clone, Copy)]
struct IntermediateTransaction {
    pub r#type: Type,
    pub client: u16,
    pub tx: u32,
    #[serde(with = "rust_decimal::serde::float_option")]
    pub amount: Option<Decimal>,
}

#[derive(Deserialize, Debug, Clone, Copy)]
// Can't use #[serde(tag = "type")] https://github.com/BurntSushi/rust-csv/issues/211
#[serde(try_from = "IntermediateTransaction")]
pub enum Transaction {
    Deposit {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Withdrawal {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Dispute {
        client: u16,
        tx: u32,
    },
    Resolve {
        client: u16,
        tx: u32,
    },
    Chargeback {
        client: u16,
        tx: u32,
    },
}

impl Transaction {
    pub fn tx(&self) -> u32 {
        use Transaction::*;

        match *self {
            Deposit { tx, .. } => tx,
            Withdrawal { tx, .. } => tx,
            Dispute { tx, .. } => tx,
            Resolve { tx, .. } => tx,
            Chargeback { tx, .. } => tx,
        }
    }

    pub fn client(&self) -> u16 {
        use Transaction::*;

        match *self {
            Deposit { client, .. } => client,
            Withdrawal { client, .. } => client,
            Dispute { client, .. } => client,
            Resolve { client, .. } => client,
            Chargeback { client, .. } => client,
        }
    }
}

impl TryFrom<IntermediateTransaction> for Transaction {
    type Error = &'static str;

    fn try_from(value: IntermediateTransaction) -> Result<Self, Self::Error> {
        use Type::*;

        let t = match value.r#type {
            Deposit => Transaction::Deposit {
                client: value.client,
                tx: value.tx,
                amount: value.amount.ok_or("Missing amount")?,
            },
            Withdrawal => Transaction::Withdrawal {
                client: value.client,
                tx: value.tx,
                amount: value.amount.ok_or("Missing amount")?,
            },
            Dispute => Transaction::Dispute {
                client: value.client,
                tx: value.tx,
            },
            Resolve => Transaction::Resolve {
                client: value.client,
                tx: value.tx,
            },
            Chargeback => Transaction::Chargeback {
                client: value.client,
                tx: value.tx,
            },
        };

        Ok(t)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DisputedState {
    Undisputed,
    Disputed,
    Resolved,
    Chargebacked,
}

pub struct TransactionRecord {
    pub amount: Decimal,
    pub client: u16,
    pub disputed: DisputedState,
}

pub struct TransactionProcessor {
    accounts: HashMap<u16, Account>,
    transactions: HashMap<u32, TransactionRecord>,
}

impl TransactionProcessor {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    pub fn process_transactions<P>(&mut self, path: P) -> Result<(), std::io::Error>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path)?;
        let mut reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(file);

        for transaction in reader.deserialize::<Transaction>().flatten() {
            // TODO: handle errors with transactions: log, notify payment partner of issues etc.
            let _ = self.process(&transaction);
        }

        Ok(())
    }

    pub fn process(&mut self, transaction: &Transaction) -> Result<(), ()> {
        if let Some(account) = self.accounts.get_mut(&transaction.client()) {
            account.process(transaction, &mut self.transactions)
        } else {
            let mut account = Account::new(transaction.client());
            let result = account.process(transaction, &mut self.transactions);

            if result.is_ok() {
                self.accounts.insert(transaction.client(), account);
            }

            result
        }
    }

    pub fn print_accounts(&self) -> Result<(), csv::Error> {
        let mut wtr = csv::Writer::from_writer(std::io::stdout());
        wtr.write_record(&["client", "available", "held", "total", "locked"])?;

        for account in self.accounts.values() {
            wtr.serialize((
                account.client,
                format!("{:.4}", account.available.round_dp(4)),
                format!("{:.4}", account.held.round_dp(4)),
                format!("{:.4}", (account.available + account.held).round_dp(4)),
                account.locked,
            ))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 1.0, Ok(()));
        test.deposit(1, 1, 2.0, Ok(()));
        test.deposit(0, 2, 4.0, Ok(()));

        test.expect(0, 5.0, 0.0, false);
        test.expect(1, 2.0, 0.0, false);

        test.run();
    }

    #[test]
    fn negative_deposit() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, -1.0, Err(()));

        test.run()
    }

    #[test]
    fn withdrawal() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.deposit(1, 1, 10.0, Ok(()));
        test.withdrawal(0, 2, 1.0, Ok(()));
        test.withdrawal(0, 3, 2.0, Ok(()));
        test.withdrawal(1, 4, 7.0, Ok(()));

        test.expect(0, 2.0, 0.0, false);
        test.expect(1, 3.0, 0.0, false);

        test.run();
    }

    #[test]
    fn withdraw_negative() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.withdrawal(0, 2, -1.0, Err(()));

        test.expect(0, 5.0, 0.0, false);

        test.run();
    }

    #[test]
    fn withdrawal_amount_greater_than_available() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 1.0, Ok(()));
        test.withdrawal(0, 2, 2.0, Err(()));

        test.expect(0, 1.0, 0.0, false);

        test.run()
    }

    #[test]
    fn dispute() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.deposit(1, 1, 10.0, Ok(()));
        test.deposit(2, 2, 15.0, Ok(()));
        test.dispute(0, 0, Ok(()));
        test.dispute(1, 1, Ok(()));

        test.expect(0, 0.0, 5.0, false);
        test.expect(1, 0.0, 10.0, false);
        test.expect(2, 15.0, 0.0, false);

        test.run();
    }

    #[test]
    fn dispute_the_same_transaction_multiple_times() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.dispute(0, 0, Ok(()));
        test.dispute(0, 0, Err(()));

        test.expect(0, 0.0, 5.0, false);

        test.run();
    }

    #[test]
    fn dispute_missing_transaction() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.dispute(0, 1, Err(()));

        test.expect(0, 5.0, 0.0, false);

        test.run();
    }

    #[test]
    fn dispute_and_disputed_have_different_clients() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.dispute(1, 0, Err(()));

        test.expect(0, 5.0, 0.0, false);

        test.run();
    }

    #[test]
    fn resolve() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.deposit(1, 1, 10.0, Ok(()));
        test.deposit(2, 2, 15.0, Ok(()));
        test.dispute(0, 0, Ok(()));
        test.dispute(1, 1, Ok(()));
        test.resolve(0, 0, Ok(()));
        test.resolve(1, 1, Ok(()));

        test.expect(0, 5.0, 0.0, false);
        test.expect(1, 10.0, 0.0, false);
        test.expect(2, 15.0, 0.0, false);

        test.run();
    }

    #[test]
    fn resolve_with_different_client() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.dispute(0, 0, Ok(()));
        test.resolve(1, 0, Err(()));

        test.expect(0, 0.0, 5.0, false);

        test.run();
    }

    #[test]
    fn resolve_undisputed_transaction() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.resolve(0, 0, Err(()));

        test.expect(0, 5.0, 0.0, false);

        test.run();
    }

    #[test]
    fn resolve_missing_transaction() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.resolve(0, 1, Err(()));

        test.expect(0, 5.0, 0.0, false);

        test.run();
    }

    #[test]
    fn resolve_transaction_that_has_been_chargebacked() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.dispute(0, 0, Ok(()));
        test.chargeback(0, 0, Ok(()));
        test.resolve(0, 0, Err(()));

        test.expect(0, 0.0, 0.0, true);

        test.run();
    }

    #[test]
    fn chargeback() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.deposit(1, 1, 10.0, Ok(()));
        test.deposit(2, 2, 15.0, Ok(()));
        test.dispute(0, 0, Ok(()));
        test.dispute(1, 1, Ok(()));
        test.chargeback(0, 0, Ok(()));
        test.chargeback(1, 1, Ok(()));

        test.expect(0, 0.0, 0.0, true);
        test.expect(1, 0.0, 0.0, true);
        test.expect(2, 15.0, 0.0, false);

        test.run();
    }

    #[test]
    fn chargeback_with_different_client() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.dispute(0, 0, Ok(()));
        test.chargeback(1, 0, Err(()));

        test.expect(0, 0.0, 5.0, false);

        test.run();
    }

    #[test]
    fn chargeback_undisputed_transaction() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.chargeback(0, 0, Err(()));

        test.expect(0, 5.0, 0.0, false);

        test.run();
    }

    #[test]
    fn chargeback_missing_transaction() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.chargeback(0, 1, Err(()));

        test.expect(0, 5.0, 0.0, false);

        test.run();
    }

    #[test]
    fn chargeback_transaction_that_has_been_resolved() {
        let mut test = TransactionTest::default();

        test.deposit(0, 0, 5.0, Ok(()));
        test.dispute(0, 0, Ok(()));
        test.resolve(0, 0, Ok(()));
        test.chargeback(0, 0, Err(()));

        test.expect(0, 5.0, 0.0, false);

        test.run();
    }

    #[derive(Debug, Default)]
    struct TransactionTest {
        transactions: Vec<Transaction>,
        transaction_results: Vec<Result<(), ()>>,
        expected: HashMap<u16, Account>,
    }

    impl TransactionTest {
        fn deposit(
            &mut self,
            client: u16,
            tx: u32,
            amount: f32,
            transaction_result: Result<(), ()>,
        ) {
            self.transactions.push(Transaction::Deposit {
                client,
                tx,
                amount: Decimal::from_f32_retain(amount).unwrap(),
            });
            self.transaction_results.push(transaction_result);
        }

        fn withdrawal(
            &mut self,
            client: u16,
            tx: u32,
            amount: f32,
            transaction_result: Result<(), ()>,
        ) {
            self.transactions.push(Transaction::Withdrawal {
                client,
                tx,
                amount: Decimal::from_f32_retain(amount).unwrap(),
            });
            self.transaction_results.push(transaction_result);
        }

        fn dispute(&mut self, client: u16, tx: u32, transaction_result: Result<(), ()>) {
            self.transactions.push(Transaction::Dispute { client, tx });
            self.transaction_results.push(transaction_result);
        }

        fn resolve(&mut self, client: u16, tx: u32, transaction_result: Result<(), ()>) {
            self.transactions.push(Transaction::Resolve { client, tx });
            self.transaction_results.push(transaction_result);
        }

        fn chargeback(&mut self, client: u16, tx: u32, transaction_result: Result<(), ()>) {
            self.transactions
                .push(Transaction::Chargeback { client, tx });
            self.transaction_results.push(transaction_result);
        }

        fn expect(&mut self, client: u16, available: f32, held: f32, locked: bool) {
            self.expected.insert(
                client,
                Account {
                    client,
                    available: Decimal::from_f32_retain(available).unwrap(),
                    held: Decimal::from_f32_retain(held).unwrap(),
                    locked,
                },
            );
        }

        fn run(&self) {
            let mut transaction_processor = TransactionProcessor::new();

            self.transactions
                .iter()
                .zip(self.transaction_results.iter())
                .for_each(|(transaction, expected_result)| {
                    let actual = transaction_processor.process(transaction);
                    assert_eq!(&actual, expected_result);
                });

            assert_eq!(transaction_processor.accounts, self.expected);
        }
    }
}
