Rust 1.63.0, Windows

Checked against `cargo fmt --check`, `cargo clippy --all-targets` and `cargo audit`.

Handles all transaction types.

Unit tests test the login the the TransactionProcessor.

Integration tests run the bin with the .csv's in the tests folder and asserts on the stdout/stderr and exit code. The integration tests test that the bin can be ran with the right API, various different types of file are proccessed correctly and that the output from the bin looks correct - right headers, client details and right precision.

If the file argument is not provided or the file doesn't exist - exit with exit code 1 and logs to stderr.

I ignore badly formatted records.

If transactions fail / should be ignored I return errors.

### Rules I added because I think its what a ATM/bank would do
- You can't despute a transaction multiple times.
- You can't resolve or chargeback a disputed transaction if the transaction has already been in resolved or chargebacked
- No negative deposits/withdrawals
- If a client doen't exist and the transaction fails then I don't create the client.
- Transaction client IDs must match the transactions they depend on. A disputed transaction's client must be the same as it's deposit and resolve/chargeback must be the same as its dispute.
