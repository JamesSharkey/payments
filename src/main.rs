mod account;
mod transaction;

use std::io::ErrorKind;
use transaction::TransactionProcessor;

fn main() -> Result<(), std::io::Error> {
    let filename = filename()?;
    let mut transaction_processor = TransactionProcessor::new();
    transaction_processor.process_transactions(filename)?;
    transaction_processor.print_accounts()?;

    Ok(())
}

fn filename() -> Result<String, std::io::Error> {
    std::env::args()
        .nth(1)
        .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidData, "Missing filepath argument"))
}
