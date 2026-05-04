pub mod hex_serde;

use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};
use secp256k1::SecretKey;

use crate::data::{Account, CkbScript};
use crate::data::cell::CkbCell;
use crate::network::transaction::{CKBTransaction, CellDep, CellInput, CellOutput, OutPoint, WitnessArgs};
use crate::network::consensus::MockLedger;

#[derive(Parser)]
#[command(name = "ckbuilder", about = "CKB mock network tool", version)]
pub struct Cli {
    /// Path to the persistent ledger state file
    #[arg(long, default_value = "src/ledger/ledger.json", global = true)]
    pub ledger: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate an account keypair from a secret key
    Account(AccountArgs),
    /// Derive a bech32 CKB address from a lock script
    Address(AddressArgs),
    /// Manage the persistent mock ledger
    #[command(subcommand)]
    Ledger(LedgerCmd),
    /// Build, sign, and inspect transactions
    #[command(subcommand)]
    Tx(TxCmd),
}

// ── Account ──────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct AccountArgs {
    /// 32-byte secret key as hex (omit to generate a random one)
    #[arg(long)]
    pub secret: Option<String>,
}

// ── Address ──────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct AddressArgs {
    #[arg(long)]
    pub code_hash: String,
    #[arg(long)]
    pub hash_type: u8,
    /// 20-byte pubkey hash as hex
    #[arg(long)]
    pub args: String,
}

// ── Ledger ───────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum LedgerCmd {
    /// Add a live cell to the ledger
    Birth(BirthArgs),
    /// Remove (spend) a cell from the ledger
    Kill(OutPointArgs),
    /// Check whether a cell is live
    Status(OutPointArgs),
    /// List all live cells
    List,
}

#[derive(Args)]
pub struct BirthArgs {
    #[arg(long)]
    pub tx_hash: String,
    #[arg(long)]
    pub index: u32,
    /// Capacity in shannons (1 CKB = 100_000_000 shannons)
    #[arg(long)]
    pub capacity: u64,
    #[arg(long)]
    pub lock_code_hash: String,
    #[arg(long)]
    pub lock_hash_type: u8,
    /// 20-byte pubkey hash as hex
    #[arg(long)]
    pub lock_args: String,
}

#[derive(Args)]
pub struct OutPointArgs {
    #[arg(long)]
    pub tx_hash: String,
    #[arg(long)]
    pub index: u32,
}

// ── Tx ───────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum TxCmd {
    /// Build an unsigned single-input / single-output transaction
    Build(BuildArgs),
    /// Sign a transaction JSON file with a private key
    Sign(SignArgs),
    /// Print the transaction hash (excludes witnesses)
    Hash(TxFileArg),
    /// Run validate_spend check on a transaction
    Validate(ValidateArgs),
}

#[derive(Args)]
pub struct BuildArgs {
    /// Input cell: outpoint tx hash (hex)
    #[arg(long)]
    pub from_tx_hash: String,
    /// Input cell: outpoint index
    #[arg(long)]
    pub from_index: u32,
    /// Output capacity in shannons
    #[arg(long)]
    pub to_capacity: u64,
    /// Output lock code_hash (hex)
    #[arg(long)]
    pub to_code_hash: String,
    /// Output lock hash_type (0=Data 1=Type 2=Data1 4=Data2)
    #[arg(long)]
    pub to_hash_type: u8,
    /// Output lock args — recipient pubkey hash (20-byte hex)
    #[arg(long)]
    pub to_args: String,
    /// Where to write the unsigned tx JSON
    #[arg(long, default_value = "src/ledger/txs/tx.json")]
    pub out: PathBuf,
}

#[derive(Args)]
pub struct SignArgs {
    /// Path to the unsigned tx JSON
    #[arg(long)]
    pub tx: PathBuf,
    /// 32-byte private key as hex
    #[arg(long)]
    pub secret: String,
    /// Output path (default: overwrites --tx)
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Args)]
pub struct TxFileArg {
    #[arg(long)]
    pub tx: PathBuf,
}

#[derive(Args)]
pub struct ValidateArgs {
    #[arg(long)]
    pub tx: PathBuf,
    /// Which input index to validate (default: 0)
    #[arg(long, default_value = "0")]
    pub input_index: usize,
}

// ── Dispatch ─────────────────────────────────────────────────────────────────

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Account(args) => cmd_account(args),
        Commands::Address(args) => cmd_address(args),
        Commands::Ledger(cmd) => cmd_ledger(cmd, &cli.ledger),
        Commands::Tx(cmd) => cmd_tx(cmd, &cli.ledger),
    }
}

fn cmd_account(args: AccountArgs) -> Result<()> {
    let secret: [u8; 32] = match args.secret {
        Some(ref hex) => parse32(hex)?,
        None => {
            use rand::{Rng, rng};
            let mut r = rng();
            let mut b = [0u8; 32];
            r.fill_bytes(&mut b);
            b
        }
    };
    let account = Account::from_secret(secret);
    println!("secret_key:  {}", hex::encode(secret));
    println!("pubkey_hash: {}", hex::encode(account.pubkey_hash));
    Ok(())
}

fn cmd_address(args: AddressArgs) -> Result<()> {
    let lock = CkbScript {
        code_hash: parse32(&args.code_hash)?,
        hash_type: args.hash_type,
        args: parse20(&args.args)?,
    };
    let addr = CkbCell::create_address(lock)?;
    println!("{}", addr);
    Ok(())
}

fn cmd_ledger(cmd: LedgerCmd, path: &Path) -> Result<()> {
    match cmd {
        LedgerCmd::Birth(args) => {
            let mut ledger = MockLedger::load(path)?;
            let op = OutPoint { tx_hash: parse32(&args.tx_hash)?, index: args.index };
            let lock = CkbScript {
                code_hash: parse32(&args.lock_code_hash)?,
                hash_type: args.lock_hash_type,
                args: parse20(&args.lock_args)?,
            };
            ledger.birth_cell(&op, CellOutput { capacity: args.capacity, lock_script: lock, type_script: None })?;
            ledger.save(path)?;
            println!("birthed  {}:{}", hex::encode(op.tx_hash), op.index);
        }
        LedgerCmd::Kill(args) => {
            let mut ledger = MockLedger::load(path)?;
            let op = OutPoint { tx_hash: parse32(&args.tx_hash)?, index: args.index };
            ledger.kill_cell(&op)?;
            ledger.save(path)?;
            println!("killed   {}:{}", hex::encode(op.tx_hash), op.index);
        }
        LedgerCmd::Status(args) => {
            let ledger = MockLedger::load(path)?;
            let op = OutPoint { tx_hash: parse32(&args.tx_hash)?, index: args.index };
            println!("{}", if ledger.is_live(&op) { "live" } else { "dead" });
        }
        LedgerCmd::List => {
            let ledger = MockLedger::load(path)?;
            if ledger.live_cell.is_empty() {
                println!("(no live cells)");
            } else {
                for (op, cell) in &ledger.live_cell {
                    println!(
                        "{}:{}  capacity={}  lock={}:{}:{}",
                        hex::encode(op.tx_hash),
                        op.index,
                        cell.capacity,
                        hex::encode(cell.lock_script.code_hash),
                        cell.lock_script.hash_type,
                        hex::encode(cell.lock_script.args),
                    );
                }
            }
        }
    }
    Ok(())
}

fn cmd_tx(cmd: TxCmd, ledger_path: &Path) -> Result<()> {
    match cmd {
        TxCmd::Build(args) => {
            let tx = CKBTransaction {
                version: 0,
                cell_deps: vec![],
                header_deps: [0u8; 32],
                inputs: vec![CellInput {
                    previous_outpoint: OutPoint { tx_hash: parse32(&args.from_tx_hash)?, index: args.from_index },
                    since: 0,
                }],
                witnesses: vec![WitnessArgs { lock: None, input_type: None, output_type: None }],
                outputs: vec![CellOutput {
                    capacity: args.to_capacity,
                    lock_script: CkbScript {
                        code_hash: parse32(&args.to_code_hash)?,
                        hash_type: args.to_hash_type,
                        args: parse20(&args.to_args)?,
                    },
                    type_script: None,
                }],
                output_data: vec![],
            };
            std::fs::write(&args.out, serde_json::to_string_pretty(&tx)?)?;
            println!("tx_hash: {}", hex::encode(tx.hash()));
            println!("written: {}", args.out.display());
        }
        TxCmd::Sign(args) => {
            let json = std::fs::read_to_string(&args.tx)?;
            let mut tx: CKBTransaction = serde_json::from_str(&json)?;
            let secret = parse32(&args.secret)?;
            let sk = SecretKey::from_byte_array(secret)?;
            let sig = tx.create_signature(sk);
            tx.witnesses[0].lock = Some(sig.to_vec());
            let out = args.out.as_deref().unwrap_or(args.tx.as_path());
            std::fs::write(out, serde_json::to_string_pretty(&tx)?)?;
            let account = Account::from_secret(secret);
            println!("pubkey_hash: {}", hex::encode(account.pubkey_hash));
            println!("tx_hash:     {}", hex::encode(tx.hash()));
            println!("written:     {}", out.display());
        }
        TxCmd::Hash(args) => {
            let json = std::fs::read_to_string(&args.tx)?;
            let tx: CKBTransaction = serde_json::from_str(&json)?;
            println!("{}", hex::encode(tx.hash()));
        }
        TxCmd::Validate(args) => {
            let json = std::fs::read_to_string(&args.tx)?;
            let tx: CKBTransaction = serde_json::from_str(&json)?;
            let ledger = MockLedger::load(ledger_path)?;
            let cells: Vec<crate::data::cell::CkbCell> = tx.inputs.iter()
                .map(|inp| {
                    let co = ledger.live_cell.get(&inp.previous_outpoint)
                        .ok_or_else(|| anyhow!("input cell {}:{} not found in ledger",
                            hex::encode(inp.previous_outpoint.tx_hash),
                            inp.previous_outpoint.index))?;
                    Ok(crate::data::cell::CkbCell::new(co.capacity, co.lock_script))
                })
                .collect::<Result<_>>()?;
            match tx.validate_spend(args.input_index, &cells) {
                Ok(()) => println!("valid"),
                Err(e) => println!("invalid: {}", e),
            }
        }
    }
    Ok(())
}

// ── Hex helpers ───────────────────────────────────────────────────────────────

fn parse32(s: &str) -> Result<[u8; 32]> {
    hex::decode(s)?
        .try_into()
        .map_err(|_| anyhow!("expected 32-byte hex string, got {}", s))
}

fn parse20(s: &str) -> Result<[u8; 20]> {
    hex::decode(s)?
        .try_into()
        .map_err(|_| anyhow!("expected 20-byte hex string, got {}", s))
}
