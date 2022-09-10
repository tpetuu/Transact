use csv::{ReaderBuilder, Trim};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::ffi::OsString;
use std::env;
use std::{io, process};

/// Client data structure with support for serialized output
#[derive(Serialize, Debug)]
struct Client {
    #[serde(rename = "client")]
    id: u16,
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
}

/// Type describing the possible transactions supported by the engine
#[derive(Debug, Clone)]
enum Transaction {
    DEPOSIT(u16, u32, f32),
    WITHDRAWAL(u16, u32, f32),
    DISPUTE(u16, u32),
    RESOLVE(u16, u32),
    CHARGEBACK(u16, u32),
}

/// This struct holds the CSV line input, deserialized from the file
#[derive(Deserialize, Debug)]
struct OperationInput {
    #[serde(rename = "type")]
    op_type: String,
    client: u16,
    tx: u32,
    amount: Option<f32>,
}

/// Returns a positional command line argument sent to this process.
/// If there's no arguments, returns an error.
fn get_nth_arg(n: usize) -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(n) {
        None => Err(From::from(format!("Missing cmd line argument #{n}"))),
        Some(file_path) => Ok(file_path),
    }
}

fn find_client_by_id<'a>(clients: &'a mut Vec<Client>, client_id: u16) -> Option<&'a mut Client> {
    clients.iter_mut().find(|c| c.id == client_id)
}

fn is_same_tx_id(tx: &Transaction, trans_id: u32) -> bool {
    match tx {
        Transaction::DEPOSIT(_, list_trans_id, _) => *list_trans_id == trans_id,
        Transaction::WITHDRAWAL(_, list_trans_id, _) => *list_trans_id == trans_id,
        _ => false,
    }
}

fn find_operation_by_id(transactions: &Vec<Transaction>, trans_id: u32) -> Option<&Transaction> {
    transactions.iter().find(|&tx| is_same_tx_id(tx, trans_id))
}

fn remove_operation_by_id(transactions: &mut Vec<Transaction>, trans_id: u32) {
    let mut i = 0;
    while i < transactions.len() {
        if is_same_tx_id(&transactions[i], trans_id) {
            transactions.remove(i);
            break
        } else {
            i += 1;
        }
    }
}

/// Attempts to perform a disputed operation on the specified client.
/// Returns true in case of success, or false if the dispute cannot be aplied.
fn apply_dispute<'a, 'b>(client: &'a mut Client, transaction: &'b Transaction) -> bool {
    match transaction {
        Transaction::DEPOSIT(cl_id, tx_id, tx_amount) => {
            if *cl_id != client.id {
                eprintln!(
                    "DISPUTE #{} client mismatch exp:{} act:{}",
                    tx_id, client.id, cl_id
                );
                return false;
            }
            if client.locked {
                eprintln!(
                    "DISPUTE #{} can't be applied to a locked account {}",
                    tx_id, client.id
                );
                return false;
            }
            if client.available < *tx_amount {
                eprintln!(
                    "DISPUTE #{} client lacks funds {} < {}",
                    tx_id, client.available, tx_amount
                );
                return false;
            }
            client.available -= tx_amount;
            client.held += tx_amount;
        }
        Transaction::WITHDRAWAL(cl_id, tx_id, tx_amount) => {
            if *cl_id != client.id {
                eprintln!(
                    "DISPUTE #{} client mismatch exp:{} act:{}",
                    tx_id, client.id, cl_id
                );
                return false;
            }
            if client.locked {
                eprintln!(
                    "DISPUTE #{} can't be applied to a locked account {}",
                    tx_id, client.id
                );
                return false;
            }
            client.held += tx_amount;
            client.total += tx_amount;
        }
        _ => return false,
    }
    true
}

/// Attempts to resolve the disputed operation on the specified client.
/// Returns true in case of success, or false if the operation cannot be aplied.
fn apply_resolve<'a, 'b>(client: &'a mut Client, transaction: &'b Transaction) -> bool {
    match transaction {
        Transaction::DEPOSIT(cl_id, tx_id, tx_amount) => {
            if *cl_id != client.id {
                eprintln!(
                    "RESOLVE #{} client mismatch exp:{} act:{}",
                    tx_id, client.id, cl_id
                );
                return false;
            }
            if client.locked {
                eprintln!(
                    "RESOLVE #{} can't be applied to a locked account {}",
                    tx_id, client.id
                );
                return false;
            }
            assert!(client.held >= *tx_amount);
            client.held -= tx_amount;
            client.available += tx_amount;
        }
        Transaction::WITHDRAWAL(cl_id, tx_id, tx_amount) => {
            if *cl_id != client.id {
                eprintln!(
                    "RESOLVE #{} client mismatch exp:{} act:{}",
                    tx_id, client.id, cl_id
                );
                return false;
            }
            if client.locked {
                eprintln!(
                    "RESOLVE #{} can't be applied to a locked account {}",
                    tx_id, client.id
                );
                return false;
            }
            assert!(client.held >= *tx_amount); // Sanity check, shouldn't happen
            client.held -= tx_amount;
            assert!(client.total >= *tx_amount); // Sanity check, shouldn't happen
            client.total -= tx_amount;
        }
        _ => return false,
    }
    true
}

/// Applies a chargeback operation on the specified client.
/// Returns true in case of success, or false if the operation cannot be aplied.
fn apply_chargeback<'a, 'b>(client: &'a mut Client, transaction: &'b Transaction) -> bool {
    match transaction {
        Transaction::DEPOSIT(cl_id, tx_id, tx_amount) => {
            if *cl_id != client.id {
                eprintln!(
                    "CHARGEBACK #{} client mismatch exp:{} act:{}",
                    tx_id, client.id, cl_id
                );
                return false;
            }
            if client.locked {
                eprintln!(
                    "CHARGEBACK #{} can't be applied to a locked account {}",
                    tx_id, client.id
                );
                return false;
            }
            assert!(client.held >= *tx_amount); // Sanity check, shouldn't happen
            client.held -= tx_amount;
            assert!(client.total >= *tx_amount); // Sanity check, shouldn't happen
            client.total -= tx_amount;
        }
        Transaction::WITHDRAWAL(cl_id, tx_id, tx_amount) => {
            if *cl_id != client.id {
                eprintln!(
                    "Chargeback transaction #{} client mismatch exp:{} act:{}",
                    tx_id, client.id, cl_id
                );
                return false;
            }
            if client.locked {
                eprintln!(
                    "CHARGEBACK #{} can't be applied to a locked account {}",
                    tx_id, client.id
                );
                return false;
            }
            assert!(client.held >= *tx_amount); // Sanity check, shouldn't happen
            client.held -= tx_amount;
            client.available += tx_amount;
        }
        _ => return false,
    }
    client.locked = true;
    true
}

/// Processes a single transaction, while updating the list of clients, disputable operations, and disputes
fn process_transaction(
    transaction: &Transaction,
    clients: &mut Vec<Client>,
    operations: &mut Vec<Transaction>,
    disputes: &mut Vec<Transaction>,
) {
    match transaction {
        Transaction::DEPOSIT(client_id, tx_id, amount) => {
            let client = find_client_by_id(clients, *client_id);
            match client {
                Some(cl) => {
                    if cl.locked {
                        eprintln!(
                            "DEPOSIT #{} can't be applied to a locked account {}",
                            tx_id, cl.id
                        );
                        return;
                    }
                    cl.available += amount;
                    cl.total += amount;
                }
                None => {
                    // If the client is not found, neet to create a new record for it.
                    clients.push(Client {
                        id: *client_id,
                        available: *amount,
                        held: 0.0,
                        total: *amount,
                        locked: false,
                    })
                }
            }
            // Deposit is always accepted, and registered in the disputable list
            operations.push(Transaction::DEPOSIT(*client_id, *tx_id, *amount));
        }
        Transaction::WITHDRAWAL(client_id, tx_id, amount) => {
            let client = find_client_by_id(clients, *client_id);
            match client {
                Some(cl) => {
                    if cl.locked {
                        eprintln!(
                            "WITHDRAWAL #{} can't be applied to a locked account {}",
                            tx_id, cl.id
                        );
                        return;
                    }
                    if cl.available < *amount {
                        eprintln!(
                            "WITHDRAWAL #{} doesn't have enough funds ({} < {})",
                            tx_id, cl.available, amount
                        );
                        return;
                    }
                    cl.available -= *amount;
                    cl.total -= *amount;
                    // Only register the withdrawal in disputable list if it was successful
                    operations.push(transaction.clone());
                }
                None => {
                    eprintln!("WITHDRAWAL #{} unknown client {}", tx_id, client_id);
                }
            }
        }
        Transaction::DISPUTE(client_id, tx_id) => {
            let client = find_client_by_id(clients, *client_id);
            match client {
                Some(cl) => {
                    let operation = find_operation_by_id(operations, *tx_id);
                    match operation {
                        Some(dispute_tx) => {
                            if apply_dispute(cl, dispute_tx) {
                                // Remember the operation in the dispute list for later settlement
                                disputes.push(dispute_tx.clone());
                            }
                        }
                        None => {
                            eprintln!("DISPUTE transaction #{} unknown or invalid", tx_id);
                            return;
                        }
                    }
                    // Once the dispute is handled, the same operation can no longer be "challenged" again
                    remove_operation_by_id(operations, *tx_id);
                }
                None => {
                    eprintln!("DISPUTE unknown client {}", client_id);
                }
            }
        }
        Transaction::RESOLVE(client_id, tx_id) => {
            let client = find_client_by_id(clients, *client_id);
            match client {
                Some(cl) => {
                    let operation = find_operation_by_id(disputes, *tx_id);
                    match operation {
                        Some(resolve_tx) => {
                            if apply_resolve(cl, resolve_tx) {
                                // Once the dispute is resolved, the operation can no longer be "finalized" again
                                remove_operation_by_id(disputes, *tx_id);
                            }
                        }
                        None => {
                            eprintln!("RESOLVE transaction #{} unknown or invalid", tx_id);
                        }
                    }
                }
                None => {
                    eprintln!("RESOLVE unknown client {}", client_id);
                }
            }
        }
        Transaction::CHARGEBACK(client_id, tx_id) => {
            let client = find_client_by_id(clients, *client_id);
            match client {
                Some(cl) => {
                    let operation = find_operation_by_id(disputes, *tx_id);
                    match operation {
                        Some(chargeback_tx) => {
                            if apply_chargeback(cl, chargeback_tx) {
                                // Once the dispute is resolved, the operation can no longer be "finalized" again
                                remove_operation_by_id(disputes, *tx_id);
                            }
                        }
                        None => {
                            eprintln!("CHARGEBACK transaction #{} unknown or invalid", tx_id);
                        }
                    }
                }
                None => {
                    eprintln!("CHARGEBACK unknown client {}", client_id);
                }
            }
        }
    }
}

/// Processes a list of string transactions, parsed by the serde, while building a list of clients
/// according to the operations in the transaction list.
fn process_transaction_list<'a>(clients: &'a mut Vec<Client>, lst: Vec<OperationInput>) {
    let mut transactions: Vec<Transaction> = Vec::new(); // Keeps the transactions that can be disputed
    let mut disputes: Vec<Transaction> = Vec::new(); // Keeps the list of disputed transactions
    for l in lst {
        let transaction: Transaction;
        let op_str = l.op_type.as_str();
        match op_str {
            // Need to convert from string representation to an Enum
            "deposit" => match l.amount {
                Some(amount) => {
                    transaction = Transaction::DEPOSIT(l.client, l.tx, amount);
                }
                None => {
                    eprintln!("DEPOSIT #{} missing amount", l.tx);
                    continue;
                }
            },
            "withdrawal" => match l.amount {
                Some(amount) => {
                    transaction = Transaction::WITHDRAWAL(l.client, l.tx, amount);
                }
                None => {
                    eprintln!("WITHDRAWAL #{} missing amount", l.tx);
                    continue;
                }
            },
            "dispute" => {
                transaction = Transaction::DISPUTE(l.client, l.tx);
            }
            "resolve" => {
                transaction = Transaction::RESOLVE(l.client, l.tx);
            }
            "chargeback" => {
                transaction = Transaction::CHARGEBACK(l.client, l.tx);
            }
            _ => {
                eprintln!("Unknown operation: {op_str}");
                continue;
            }
        }
        process_transaction(&transaction, clients, &mut transactions, &mut disputes);
        transactions.push(transaction);
    }
}

/// Trims the float value to four digits after the decimal point
fn round_to_4th_digit(val: f32) -> f32 {
    ((val * 10000.0) as i64) as f32 / 10000.0
}


/// Builds a vector of CSV string record from the file name given in the first command line argument.
/// If the file is not found, or the file name not provided, returns an error.
fn parse_transaction_file() -> Result<Vec<OperationInput>, Box<dyn Error>> {
    let file_path = get_nth_arg(1)?;
    let mut lines: Vec<OperationInput> = Vec::new();
    let mut file_rdr = ReaderBuilder::new()
        .trim(Trim::All)
        .flexible(true)
        .from_path(file_path)?;
    let mut iter = file_rdr.deserialize();
    while let Some(result) = iter.next() {
        let mut record: OperationInput = result?;
        if record.amount.is_some() {
            record.amount = Some(round_to_4th_digit(record.amount.unwrap()));
        }
        lines.push(record);
    }
    Ok(lines)
}

/// Writes a CSV list of records corresponding to the client list in clients to stdout.
fn dump_clients(clients: &[Client]) -> Result<(), Box<dyn Error>> {
    let mut out = csv::WriterBuilder::new().from_writer(io::stdout());
    out.serialize(("client", "available", "held", "total", "locked"))?;
    for cl in clients {
        out.serialize((
            cl.id,
            round_to_4th_digit(cl.available),
            round_to_4th_digit(cl.held),
            round_to_4th_digit(cl.total),
            cl.locked,
        ))?;
    }
    out.flush()?;
    Ok(())
}

fn main() {
    let parse_res = parse_transaction_file();
    let mut clients: Vec<Client> = Vec::new();
    match parse_res {
        Ok(v) => {
            process_transaction_list(&mut clients, v);
            let dump_res = dump_clients(clients.as_slice());
            match dump_res {
                Err(err) => {
                    eprintln!("{}", err);
                    process::exit(1)
                }
                _ => (),
            }
        }
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1)
        }
    }
}
