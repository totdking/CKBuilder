pub mod hex_serde;

use anyhow::{Result, anyhow};
use clap::{Args, Parser, Subcommand};
use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::data::cell::CkbCell;
use crate::data::{Account, CkbScript};
use crate::network::rpc::{
    CkbRpcClient, DEVNET_RPC, MIN_CELL_CAPACITY, Network, SECP256K1_CODE_HASH, SECP256K1_DEP_INDEX,
    SECP256K1_DEP_TYPE, estimate_fee, select_cells,
};
use crate::network::transaction::{
    CKBTransaction, CellDep, CellInput, CellOutput, OutPoint, WitnessArgs,
};

// ── Config file ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct CkbConfig {
    network: Network,
}

/// ~/.config/ckb/config.json
fn default_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("ckb")
        .join("config.json")
}

fn load_config() -> CkbConfig {
    let path = default_config_path();
    if let Ok(json) = std::fs::read_to_string(&path)
        && let Ok(cfg) = serde_json::from_str::<CkbConfig>(&json)
    {
        return cfg;
    }
    CkbConfig {
        network: Network::Testnet,
    }
}

fn save_config(cfg: &CkbConfig) -> Result<()> {
    let path = default_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}

// ── Keypair file ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct KeyFile {
    secret_key: String,
    pubkey_hash: String,
    address_testnet: String,
    address_mainnet: String,
}

impl KeyFile {
    fn active_address(&self, network: Network) -> &str {
        match network {
            Network::Testnet | Network::Devnet => &self.address_testnet,
            Network::Mainnet => &self.address_mainnet,
        }
    }
}

/// ~/.config/ckb/key.json
fn default_keypair_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("ckb")
        .join("key.json")
}

fn load_keypair(path: &Path) -> Result<[u8; 32]> {
    let json = std::fs::read_to_string(path).map_err(|_| {
        anyhow!(
            "no keypair found at {}\nRun `ckb account new` to generate one",
            path.display()
        )
    })?;
    let kf: KeyFile =
        serde_json::from_str(&json).map_err(|e| anyhow!("keypair file is malformed: {}", e))?;
    parse32(&kf.secret_key)
}

/// Resolves a secret key from either --secret hex, --keypair path, or the default keypair file.
fn resolve_secret(keypair: Option<&Path>, secret: Option<&str>) -> Result<[u8; 32]> {
    if let Some(s) = secret {
        return parse32(s);
    }
    let path = keypair
        .map(|p| p.to_path_buf())
        .unwrap_or_else(default_keypair_path);
    load_keypair(&path)
}

// ── Hex helpers ───────────────────────────────────────────────────────────────

fn parse32(s: &str) -> Result<[u8; 32]> {
    let s = s.trim_start_matches("0x");
    hex::decode(s)?.try_into().map_err(|_| {
        anyhow!(
            "expected 32-byte hex string, got {} bytes from '{}'",
            s.len() / 2,
            s
        )
    })
}

fn parse20(s: &str) -> Result<[u8; 20]> {
    let s = s.trim_start_matches("0x");
    hex::decode(s)?.try_into().map_err(|_| {
        anyhow!(
            "expected 20-byte hex string, got {} bytes from '{}'",
            s.len() / 2,
            s
        )
    })
}

// ── hash_type parser ─────────────────────────────────────────────────────────

fn parse_hash_type(s: &str) -> Result<u8> {
    match s {
        "data" => Ok(0),
        "type" => Ok(1),
        other => Err(anyhow!(
            "unknown hash_type '{}' — use 'data' or 'type'",
            other
        )),
    }
}

// ── CKB amount parser (CKB → shannons) ───────────────────────────────────────

/// Accepts "100" or "100.5" as CKB, returns shannons (u64).
/// 1 CKB = 100,000,000 shannons. Minimum 61 CKB (a cell must cover its own storage).
fn parse_ckb_amount(s: &str) -> Result<u64> {
    let ckb: f64 = s
        .parse()
        .map_err(|_| anyhow!("invalid CKB amount '{}' — use a number like 100 or 61.5", s))?;
    if ckb <= 0.0 {
        return Err(anyhow!("amount must be positive, got {}", s));
    }
    let shannons = (ckb * 1e8).round() as u64;
    if shannons < MIN_CELL_CAPACITY {
        return Err(anyhow!(
            "amount too small: {} shannons ({} CKB) — minimum is {} shannons (61 CKB)",
            shannons,
            s,
            MIN_CELL_CAPACITY
        ));
    }
    Ok(shannons)
}

// ── Address / pubkey_hash parser ──────────────────────────────────────────────

/// Accepts a bech32m CKB address ("ckt1q..." / "ckb1q...") or a 20-byte hex string.
/// Returns the pubkey_hash (lock script args) as [u8; 20].
fn parse_addr_or_pubkey_hash(s: &str) -> Result<[u8; 20]> {
    if s.starts_with("ckt1") || s.starts_with("ckb1") {
        // Decode bech32m — payload layout: 0x00 | code_hash(32) | hash_type(1) | args(N)
        let (_, payload) =
            bech32::decode(s).map_err(|e| anyhow!("invalid bech32m address '{}': {}", s, e))?;
        if payload.len() < 54 {
            return Err(anyhow!(
                "address payload too short ({} bytes); expected at least 54 (1 + 32 + 1 + 20)",
                payload.len()
            ));
        }
        if payload[0] != 0x00 {
            return Err(anyhow!(
                "only full-format CKB addresses are supported (payload must start with 0x00)"
            ));
        }
        let mut args = [0u8; 20];
        args.copy_from_slice(&payload[34..54]);
        return Ok(args);
    }
    // Fall through: treat as 20-byte hex
    parse20(s).map_err(|_| anyhow!(
        "invalid address or pubkey_hash '{}'\n  expected: bech32m address (ckt1... / ckb1...) or 20-byte hex (0x...)",
        s
    ))
}

// ── Outpoint string parser ("txhash:index") ───────────────────────────────────

fn parse_outpoint_str(s: &str) -> Result<([u8; 32], u32)> {
    let (hash_part, index_part) = s.split_once(':').ok_or_else(|| {
        anyhow!(
            "invalid outpoint '{}' — expected format: <txhash>:<index>  e.g. 71a7ba8f...:0",
            s
        )
    })?;
    let tx_hash = parse32(hash_part)?;
    let index: u32 = index_part
        .parse()
        .map_err(|_| anyhow!("invalid outpoint index '{}' — must be a number", index_part))?;
    Ok((tx_hash, index))
}

// ── CLI root ──────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "ckb", about = "CKB developer toolkit", version)]
pub struct Cli {
    /// Use local offckb devnet (http://localhost:8114) for this command only
    #[arg(long, short = 'd', global = true)]
    pub devnet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage your local keypair (~/.config/ckb/key.json)
    #[command(subcommand)]
    Account(AccountCmd),
    /// Advanced: derive a bech32m address from arbitrary lock script parameters
    Address(AddressArgs),
    /// Request testnet CKB from the Nervos faucet (testnet only)
    Airdrop(AirdropArgs),
    /// Check the CKB balance of any address on the active network
    Balance(BalanceArgs),
    /// Inspect a live cell by outpoint (capacity, lock, type script, data)
    Cell(CellArgs),
    /// Fetch a block by number or hash
    Block(BlockArgs),
    /// Get or set the active network (testnet / mainnet / devnet)
    #[command(subcommand)]
    Config(ConfigCmd),
    /// Call any CKB JSON-RPC method directly
    Rpc(RpcArgs),
    /// Show the current chain tip (block number and hash)
    Tip(TipArgs),
    /// Build, sign, inspect, and broadcast transactions
    #[command(subcommand)]
    Tx(TxCmd),
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum ConfigCmd {
    /// Show the active network and config file path
    Get,
    /// Switch the active network
    Set(ConfigSetArgs),
}

#[derive(Args)]
pub struct ConfigSetArgs {
    /// Network to activate: testnet, mainnet, or devnet (offckb localhost:8114)
    #[arg(value_parser = ["testnet", "mainnet", "devnet"])]
    pub network: String,
}

// ── Account ──────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum AccountCmd {
    /// Generate a keypair and save it to disk (default: ~/.config/ckb/key.json)
    New(AccountNewArgs),
    /// Show keypair info or look up any CKB account by address / pubkey_hash
    Show(AccountShowArgs),
}

#[derive(Args)]
pub struct AccountNewArgs {
    /// Import an existing secret key (32-byte hex) instead of generating a new one
    #[arg(long)]
    pub secret: Option<String>,
    /// Save keypair to this path instead of the default (~/.config/ckb/key.json)
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Overwrite an existing keypair file
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Args)]
pub struct AccountShowArgs {
    /// Any CKB address (ckt1q...) or pubkey_hash (0x...) to look up.
    /// Omit to show your local saved keypair.
    pub addr: Option<String>,
    /// Path to a specific keypair file (default: ~/.config/ckb/key.json)
    #[arg(long)]
    pub keypair: Option<PathBuf>,
}

// ── Balance ──────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct BalanceArgs {
    /// CKB address (ckt1q...) or pubkey_hash (0x...) to query.
    /// Omit to use your saved keypair (~/.config/ckb/key.json).
    pub addr: Option<String>,
    /// Use a keypair file to derive the address to query
    #[arg(long)]
    pub keypair: Option<PathBuf>,
    /// Use a raw secret key (32-byte hex) to derive the address to query
    #[arg(long)]
    pub secret: Option<String>,
    /// Print each individual live cell (outpoint + capacity)
    #[arg(long)]
    pub utxos: bool,
    /// Override the RPC endpoint (defaults to active network)
    #[arg(long)]
    pub rpc: Option<String>,
}

// ── Airdrop ──────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct AirdropArgs {
    /// Amount of testnet CKB to request. Must be 10000, 100000, or 300000. Defaults to 10000.
    pub amount: Option<String>,
    /// Recipient testnet address (ckt1q...). Omit to use your saved keypair address.
    #[arg(long)]
    pub address: Option<String>,
}

// ── Address ──────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct AddressArgs {
    #[arg(long)]
    pub code_hash: String,
    /// Script hash type: "data" (blake2b of binary) or "type" (type script hash)
    #[arg(long, value_parser = ["data", "type"])]
    pub hash_type: String,
    #[arg(long)]
    pub args: String,
}

// ── Cell ─────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct CellArgs {
    /// Outpoint to inspect, format: <txhash>:<index>  e.g. 0xabc...:0
    pub outpoint: String,
    /// Also fetch and display cell data (may be large)
    #[arg(long)]
    pub data: bool,
    /// Override the RPC endpoint (defaults to active network)
    #[arg(long)]
    pub rpc: Option<String>,
}

// ── Tip ──────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct TipArgs {
    /// Override the RPC endpoint (defaults to active network)
    #[arg(long)]
    pub rpc: Option<String>,
}

// ── Block ─────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct BlockArgs {
    /// Block number (decimal) or block hash (0x-prefixed 32-byte hex)
    pub target: String,
    /// Override the RPC endpoint (defaults to active network)
    #[arg(long)]
    pub rpc: Option<String>,
}

// ── Rpc ──────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct RpcArgs {
    /// JSON-RPC method name (e.g. get_tip_block_number, get_live_cells)
    pub method: String,
    /// Parameters as a JSON array string (default: [])
    pub params: Option<String>,
    /// Override the RPC endpoint (defaults to active network)
    #[arg(long)]
    pub rpc: Option<String>,
}

// ── Tx ───────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum TxCmd {
    /// Build an unsigned secp256k1 transaction and write it to a JSON file
    Build(BuildArgs),
    /// Sign any unsigned CKB transaction JSON with a private key
    Sign(SignArgs),
    /// Broadcast a signed transaction JSON file to the active network
    Broadcast(BroadcastArgs),
    /// Pretty-print a transaction JSON file (inputs, outputs, scripts, hash)
    Decode(TxFileArg),
    /// Print the CKB-node-compatible tx hash of a transaction JSON file
    Hash(TxFileArg),
    /// Build, sign, and broadcast a CKB transfer in one step
    Send(SendArgs),
    /// Check the on-chain status of a transaction (pending / committed / rejected)
    Status(TxStatusArgs),
}

#[derive(Args)]
pub struct TxStatusArgs {
    /// Transaction hash (32-byte hex, with or without 0x prefix)
    pub tx_hash: String,
    /// Override the RPC endpoint (defaults to active network)
    #[arg(long)]
    pub rpc: Option<String>,
}

#[derive(Args)]
pub struct BuildArgs {
    /// Input cell(s) to spend, format: <txhash>:<index>. Repeat for multiple inputs.
    /// e.g. --from abc123...:0 --from def456...:1
    #[arg(long, num_args = 1..)]
    pub from: Vec<String>,
    /// Recipient: CKB address (ckt1q...) or pubkey_hash (0x...)
    #[arg(long)]
    pub to: String,
    /// Amount to send in CKB (e.g. 100 or 61.5). Minimum 61 CKB.
    #[arg(long)]
    pub amount: String,
    /// Where to send change (default: back to the owner of the --from cell).
    /// Use this to redirect leftover capacity to a different address.
    #[arg(long)]
    pub change_to: Option<String>,
    /// Output file for the unsigned transaction JSON
    #[arg(long, default_value = "tx.json")]
    pub out: PathBuf,
}

#[derive(Args)]
pub struct SignArgs {
    /// Path to an unsigned transaction JSON file
    #[arg(long)]
    pub tx: PathBuf,
    /// Secret key as 32-byte hex (alternative to --keypair)
    #[arg(long)]
    pub secret: Option<String>,
    /// Path to a keypair JSON file (alternative to --secret; default: ~/.config/ckb/key.json)
    #[arg(long)]
    pub keypair: Option<PathBuf>,
    /// Optional: assert that the derived pubkey_hash matches this 20-byte hex.
    /// Fails early with a clear message if this key does not own the input cells.
    #[arg(long)]
    pub assert_owner: Option<String>,
    /// Write the signed tx to this file (default: overwrites --tx in place)
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Args)]
pub struct TxFileArg {
    #[arg(long)]
    pub tx: PathBuf,
}

#[derive(Args)]
pub struct BroadcastArgs {
    /// Path to a signed transaction JSON file
    #[arg(long)]
    pub tx: PathBuf,
    /// Override the RPC endpoint (defaults to active network)
    #[arg(long)]
    pub rpc: Option<String>,
}

#[derive(Args)]
pub struct SendArgs {
    /// Secret key as 32-byte hex (alternative to --keypair)
    #[arg(long)]
    pub secret: Option<String>,
    /// Path to a keypair JSON file (default: ~/.config/ckb/key.json)
    #[arg(long)]
    pub keypair: Option<PathBuf>,
    /// Recipient: CKB address (ckt1q...) or pubkey_hash (0x...)
    #[arg(long)]
    pub to: String,
    /// Amount in CKB (e.g. 100 or 61.5). Minimum 61 CKB.
    #[arg(long)]
    pub amount: String,
    /// Transaction fee in shannons (0 = auto-estimate based on tx size)
    #[arg(long, default_value_t = 0)]
    pub fee: u64,
    /// Override the RPC endpoint (defaults to active network)
    #[arg(long)]
    pub rpc: Option<String>,
}

// ── Dispatch ─────────────────────────────────────────────────────────────────

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Account(cmd) => cmd_account(cmd, cli.devnet),
        Commands::Address(args) => cmd_address(args),
        Commands::Airdrop(args) => cmd_airdrop(args),
        Commands::Balance(args) => cmd_balance(args, cli.devnet),
        Commands::Block(args) => cmd_block(args, cli.devnet),
        Commands::Cell(args) => cmd_cell(args, cli.devnet),
        Commands::Config(cmd) => cmd_config(cmd),
        Commands::Rpc(args) => cmd_rpc(args, cli.devnet),
        Commands::Tip(args) => cmd_tip(args, cli.devnet),
        Commands::Tx(cmd) => cmd_tx(cmd, cli.devnet),
    }
}

fn active_rpc(cfg: &CkbConfig, rpc_override: Option<&str>, devnet_flag: bool) -> String {
    if devnet_flag {
        return DEVNET_RPC.to_string();
    }
    rpc_override
        .map(|s| s.to_string())
        .unwrap_or_else(|| cfg.network.rpc_url().to_string())
}

fn active_network(cfg: &CkbConfig, devnet_flag: bool) -> Network {
    if devnet_flag {
        Network::Devnet
    } else {
        cfg.network
    }
}

fn cmd_airdrop(args: AirdropArgs) -> Result<()> {
    const FAUCET_URL: &str = "https://faucet-api.nervos.org/claim_events";
    const VALID_AMOUNTS: &[&str] = &["10000", "100000", "300000"];
    const FAUCET_DEFAULT_AMOUNT: &str = "10000";

    // Resolve the recipient address
    let address = match args.address {
        Some(ref addr) => {
            if !addr.starts_with("ckt1") {
                return Err(anyhow!(
                    "airdrop is testnet-only — address must start with 'ckt1', got '{}'",
                    addr
                ));
            }
            addr.clone()
        }
        None => {
            // Load from saved keypair
            let kp_path = default_keypair_path();
            let json = std::fs::read_to_string(&kp_path).map_err(|_| {
                anyhow!(
                    "no keypair found at {}\nRun `ckb account new` to generate one, or pass --address",
                    kp_path.display()
                )
            })?;
            let kf: KeyFile = serde_json::from_str(&json)
                .map_err(|e| anyhow!("keypair file is malformed: {}", e))?;
            kf.address_testnet.clone()
        }
    };

    let amount_str = args
        .amount
        .as_deref()
        .unwrap_or(FAUCET_DEFAULT_AMOUNT)
        .to_string();

    if !VALID_AMOUNTS.contains(&amount_str.as_str()) {
        return Err(anyhow!(
            "invalid amount '{}' — the faucet only accepts: {}",
            amount_str,
            VALID_AMOUNTS.join(", ")
        ));
    }

    println!("requesting {} CKB from faucet ...", amount_str);
    println!("address:  {}", address);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let body = serde_json::json!({
        "claim_event": {
            "address_hash": address,
            "amount": amount_str
        }
    });

    let resp = client
        .post(FAUCET_URL)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .map_err(|e| anyhow!("failed to reach faucet: {}", e))?;

    let status = resp.status();
    let raw = resp
        .text()
        .map_err(|e| anyhow!("failed to read faucet response: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&raw).map_err(|_| {
        anyhow!(
            "faucet returned a non-JSON response (HTTP {}):\n{}",
            status,
            raw.trim()
        )
    })?;

    if status.is_success() {
        let data = &json["data"];
        let claimed = data["amount"].as_str().unwrap_or(&amount_str);
        let claim_status = data["status"].as_str().unwrap_or("pending");
        println!("status:   {}", claim_status);
        println!("amount:   {} CKB", claimed);
        println!(
            "explorer: {}/address/{}",
            Network::Testnet.explorer_base(),
            address
        );
        if claim_status == "pending" {
            println!("note:     the faucet queues claims — CKB will arrive within a few minutes");
        }
    } else {
        // Surface faucet error message
        let msg = json
            .get("error_message")
            .or_else(|| json.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        let errors = json
            .get("errors")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        if errors.is_null() {
            return Err(anyhow!("faucet error (HTTP {}): {}", status, msg));
        }
        return Err(anyhow!(
            "faucet error (HTTP {}): {} — {}",
            status,
            msg,
            errors
        ));
    }

    Ok(())
}

fn cmd_config(cmd: ConfigCmd) -> Result<()> {
    match cmd {
        ConfigCmd::Get => {
            let cfg = load_config();
            println!("network:     {}", cfg.network);
            println!("rpc:         {}", cfg.network.rpc_url());
            println!("config file: {}", default_config_path().display());
        }
        ConfigCmd::Set(args) => {
            let network = match args.network.as_str() {
                "testnet" => Network::Testnet,
                "mainnet" => Network::Mainnet,
                "devnet" => Network::Devnet,
                other => {
                    return Err(anyhow!(
                        "unknown network '{}' — use testnet, mainnet, or devnet",
                        other
                    ));
                }
            };
            save_config(&CkbConfig { network })?;
            println!("network set to: {}", network);
            println!("rpc endpoint:   {}", network.rpc_url());
            if network == Network::Devnet {
                println!(
                    "note:           start offckb with `npx offckb node` to run a local CKB devnet"
                );
            }
        }
    }
    Ok(())
}

fn cmd_account(cmd: AccountCmd, devnet_flag: bool) -> Result<()> {
    match cmd {
        AccountCmd::New(args) => {
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

            let out = args.out.unwrap_or_else(default_keypair_path);

            if out.exists() && !args.force {
                return Err(anyhow!(
                    "keypair already exists at {}\nUse --force to overwrite",
                    out.display()
                ));
            }

            let account = Account::from_secret(secret);
            let lock = CkbScript {
                code_hash: SECP256K1_CODE_HASH,
                hash_type: 1,
                args: account.pubkey_hash,
            };

            let address_testnet = CkbCell::create_address(lock, Network::Testnet)?;
            let address_mainnet = CkbCell::create_address(lock, Network::Mainnet)?;

            let kf = KeyFile {
                secret_key: hex::encode(secret),
                pubkey_hash: hex::encode(account.pubkey_hash),
                address_testnet: address_testnet.clone(),
                address_mainnet: address_mainnet.clone(),
            };

            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&out, serde_json::to_string_pretty(&kf)?)?;

            let cfg = load_config();
            let net = active_network(&cfg, devnet_flag);
            println!("keypair saved:      {}", out.display());
            println!("pubkey_hash:        0x{}", hex::encode(account.pubkey_hash));
            println!("address (testnet):  {}", address_testnet);
            println!("address (mainnet):  {}", address_mainnet);
            println!("active network:     {} → {}", net, kf.active_address(net));
            println!();
            println!(
                "WARNING: keep {} secret — do not commit it to git",
                out.display()
            );
        }

        AccountCmd::Show(args) => {
            // If a positional address / pubkey_hash is given, look it up without needing a keypair
            if let Some(ref addr_str) = args.addr {
                let pubkey_hash = parse_addr_or_pubkey_hash(addr_str)?;
                let lock = CkbScript {
                    code_hash: SECP256K1_CODE_HASH,
                    hash_type: 1,
                    args: pubkey_hash,
                };
                let address_testnet = CkbCell::create_address(lock, Network::Testnet)?;
                let address_mainnet = CkbCell::create_address(lock, Network::Mainnet)?;
                println!("pubkey_hash:       0x{}", hex::encode(pubkey_hash));
                println!("address (testnet): {}", address_testnet);
                println!("address (mainnet): {}", address_mainnet);
                println!(
                    "note:              use `ckb balance {}` to check the balance",
                    addr_str
                );
                return Ok(());
            }

            // No positional arg — show the local keypair file
            let path = args.keypair.unwrap_or_else(default_keypair_path);
            let json = std::fs::read_to_string(&path).map_err(|_| {
                anyhow!(
                    "no keypair at {}\nRun `ckb account new` first",
                    path.display()
                )
            })?;
            let kf: KeyFile = serde_json::from_str(&json)?;
            let cfg = load_config();
            let net = active_network(&cfg, devnet_flag);
            println!("keypair:           {}", path.display());
            println!("pubkey_hash:       0x{}", kf.pubkey_hash);
            println!("network:           {}", net);
            println!("address (active):  {}", kf.active_address(net));
            println!("address (testnet): {}", kf.address_testnet);
            println!("address (mainnet): {}", kf.address_mainnet);
        }
    }
    Ok(())
}

fn cmd_address(args: AddressArgs) -> Result<()> {
    let cfg = load_config();
    let lock = CkbScript {
        code_hash: parse32(&args.code_hash)?,
        hash_type: parse_hash_type(&args.hash_type)?,
        args: parse20(&args.args)?,
    };
    let addr = CkbCell::create_address(lock, cfg.network)?;
    println!("{}", addr);
    Ok(())
}

fn cmd_balance(args: BalanceArgs, devnet_flag: bool) -> Result<()> {
    let cfg = load_config();
    let rpc_url = active_rpc(&cfg, args.rpc.as_deref(), devnet_flag);
    let net = active_network(&cfg, devnet_flag);

    // Resolve pubkey_hash: positional address/hex → key derivation → default keypair
    let pubkey_hash: [u8; 20] = if let Some(ref addr) = args.addr {
        parse_addr_or_pubkey_hash(addr)?
    } else {
        let secret = resolve_secret(args.keypair.as_deref(), args.secret.as_deref())?;
        Account::from_secret(secret).pubkey_hash
    };

    let rpc = CkbRpcClient::new(&rpc_url);

    println!("network:     {}", net);
    println!("pubkey_hash: 0x{}", hex::encode(pubkey_hash));
    println!("querying:    {}", rpc_url);

    let cells = rpc.get_live_cells(pubkey_hash)?;
    let total: u64 = cells.iter().map(|c| c.capacity).sum();
    println!("cells:       {}", cells.len());
    println!(
        "balance:     {} CKB  ({} shannons)",
        total as f64 / 1e8,
        total
    );

    if args.utxos && !cells.is_empty() {
        println!();
        println!(
            "{:<6}  {:<68}  {:>18}",
            "#", "outpoint (txhash:index)", "capacity (shannons)"
        );
        println!("{}", "-".repeat(96));
        for (i, cell) in cells.iter().enumerate() {
            println!(
                "{:<6}  0x{}:{:<4}  {:>18}",
                i,
                hex::encode(cell.out_point.tx_hash),
                cell.out_point.index,
                cell.capacity
            );
        }
    }
    Ok(())
}

fn cmd_tx(cmd: TxCmd, devnet_flag: bool) -> Result<()> {
    match cmd {
        TxCmd::Build(args) => {
            let cfg = load_config();
            let net = active_network(&cfg, devnet_flag);
            let rpc_url = active_rpc(&cfg, None, devnet_flag);
            let to_pubkey_hash = parse_addr_or_pubkey_hash(&args.to)?;
            let amount = parse_ckb_amount(&args.amount)?;

            let rpc = CkbRpcClient::new(&rpc_url);

            // Fetch all input cells
            let mut inputs: Vec<CellInput> = Vec::new();
            let mut total_input_capacity = 0u64;
            let mut first_sender_lock_args: Option<[u8; 20]> = None;

            for from_str in &args.from {
                let (tx_hash, index) = parse_outpoint_str(from_str)?;
                let outpoint = OutPoint { tx_hash, index };
                let (cap, lock_args) = rpc.get_cell_info(&outpoint)?;
                total_input_capacity = total_input_capacity
                    .checked_add(cap)
                    .ok_or_else(|| anyhow!("input capacity overflow"))?;
                if first_sender_lock_args.is_none() {
                    first_sender_lock_args = Some(lock_args);
                }
                inputs.push(CellInput {
                    previous_outpoint: outpoint,
                    since: 0,
                });
            }

            let sender_lock_args = first_sender_lock_args.unwrap();
            let change_lock_args = if let Some(ref ct) = args.change_to {
                parse_addr_or_pubkey_hash(ct)?
            } else {
                sender_lock_args
            };

            // Estimate fee assuming 2 outputs (with change); adjust if change is absorbed
            let fee = estimate_fee(inputs.len(), 2);

            if total_input_capacity < amount + fee {
                return Err(anyhow!(
                    "inputs total {} shannons ({:.8} CKB) — not enough to send {} shannons ({:.8} CKB) + fee {} shannons",
                    total_input_capacity,
                    total_input_capacity as f64 / 1e8,
                    amount,
                    amount as f64 / 1e8,
                    fee,
                ));
            }

            let raw_change = total_input_capacity - amount - fee;
            let (change, actual_fee) = if raw_change > 0 && raw_change < MIN_CELL_CAPACITY {
                (0u64, total_input_capacity - amount) // absorb dust into fee
            } else {
                (raw_change, fee)
            };

            let mut outputs = vec![CellOutput {
                capacity: amount,
                lock_script: CkbScript {
                    code_hash: SECP256K1_CODE_HASH,
                    hash_type: 1,
                    args: to_pubkey_hash,
                },
                type_script: None,
            }];
            if change > 0 {
                outputs.push(CellOutput {
                    capacity: change,
                    lock_script: CkbScript {
                        code_hash: SECP256K1_CODE_HASH,
                        hash_type: 1,
                        args: change_lock_args,
                    },
                    type_script: None,
                });
            }

            let witnesses: Vec<WitnessArgs> = (0..inputs.len())
                .map(|_| WitnessArgs {
                    lock: None,
                    input_type: None,
                    output_type: None,
                })
                .collect();

            let tx = CKBTransaction {
                version: 0,
                cell_deps: vec![CellDep {
                    outpoint: OutPoint {
                        tx_hash: net.secp256k1_dep_tx_hash(),
                        index: SECP256K1_DEP_INDEX,
                    },
                    dep_type: SECP256K1_DEP_TYPE,
                }],
                header_deps: [0u8; 32],
                inputs,
                witnesses,
                outputs,
                output_data: vec![],
            };

            std::fs::write(&args.out, serde_json::to_string_pretty(&tx)?)?;
            let change_dest = if args.change_to.is_some() {
                "→ change-to address"
            } else {
                "→ back to sender"
            };
            println!("tx hash:  0x{}", hex::encode(tx.rpc_raw_tx_hash()));
            println!(
                "inputs:   {} cell(s), {} shannons total ({:.8} CKB)",
                tx.inputs.len(),
                total_input_capacity,
                total_input_capacity as f64 / 1e8
            );
            println!(
                "send:     {} shannons ({:.8} CKB)",
                amount,
                amount as f64 / 1e8
            );
            println!(
                "change:   {} shannons ({:.8} CKB) {}",
                change,
                change as f64 / 1e8,
                change_dest
            );
            println!(
                "fee:      {} shannons ({:.8} CKB)",
                actual_fee,
                actual_fee as f64 / 1e8
            );
            println!("written:  {}", args.out.display());
            println!(
                "sign:     ckb tx sign --tx {} --keypair ~/.config/ckb/key.json",
                args.out.display()
            );
        }

        TxCmd::Sign(args) => {
            let json = std::fs::read_to_string(&args.tx)?;
            let mut tx: CKBTransaction = serde_json::from_str(&json)?;
            let secret = resolve_secret(args.keypair.as_deref(), args.secret.as_deref())?;
            let account = Account::from_secret(secret);

            if let Some(ref expected_hex) = args.assert_owner {
                let expected_args = parse_addr_or_pubkey_hash(expected_hex)?;
                if account.pubkey_hash != expected_args {
                    return Err(anyhow!(
                        "key mismatch: derived pubkey_hash 0x{} does not match --assert-owner 0x{}\n\
                         this key does not own those input cells",
                        hex::encode(account.pubkey_hash),
                        hex::encode(expected_args),
                    ));
                }
            }

            let sk = SecretKey::from_byte_array(secret)?;
            let sig = tx.create_rpc_signature(sk);
            tx.witnesses[0].lock = Some(sig.to_vec());

            let out = args.out.as_deref().unwrap_or(args.tx.as_path());
            std::fs::write(out, serde_json::to_string_pretty(&tx)?)?;
            println!("signer:    0x{}", hex::encode(account.pubkey_hash));
            println!("tx hash:   0x{}", hex::encode(tx.rpc_raw_tx_hash()));
            println!("written:   {}", out.display());
            println!("broadcast: ckb tx broadcast --tx {}", out.display());
        }

        TxCmd::Broadcast(args) => {
            let cfg = load_config();
            let rpc_url = active_rpc(&cfg, args.rpc.as_deref(), devnet_flag);
            let net = active_network(&cfg, devnet_flag);

            let json = std::fs::read_to_string(&args.tx)?;
            let tx: CKBTransaction = serde_json::from_str(&json)?;
            let tx_hash_local = tx.rpc_raw_tx_hash();

            let rpc = CkbRpcClient::new(&rpc_url);
            let tx_hash = rpc.send_transaction(tx.to_rpc_value())?;

            println!("network:  {}", net);
            println!("tx hash:  0x{}", hex::encode(tx_hash));
            println!(
                "explorer: {}/transaction/0x{}",
                net.explorer_base(),
                hex::encode(tx_hash)
            );

            if tx_hash != tx_hash_local {
                eprintln!(
                    "warning: node returned hash 0x{} but local hash was 0x{}",
                    hex::encode(tx_hash),
                    hex::encode(tx_hash_local)
                );
            }
        }

        TxCmd::Decode(args) => {
            let json = std::fs::read_to_string(&args.tx)?;
            let tx: CKBTransaction = serde_json::from_str(&json)?;

            println!("version:   {}", tx.version);

            println!("inputs ({}):", tx.inputs.len());
            for (i, inp) in tx.inputs.iter().enumerate() {
                println!(
                    "  [{}]  0x{}:{}  since={}",
                    i,
                    hex::encode(inp.previous_outpoint.tx_hash),
                    inp.previous_outpoint.index,
                    inp.since
                );
            }

            println!("outputs ({}):", tx.outputs.len());
            for (i, out) in tx.outputs.iter().enumerate() {
                let hash_type_str = match out.lock_script.hash_type {
                    0 => "data",
                    1 => "type",
                    2 => "data1",
                    4 => "data2",
                    x => Box::leak(format!("0x{:x}", x).into_boxed_str()),
                };
                let type_str = if out.type_script.is_some() {
                    "yes"
                } else {
                    "none"
                };
                println!(
                    "  [{}]  {:.8} CKB  lock: {}/0x{:.16}../args=0x{}  type: {}",
                    i,
                    out.capacity as f64 / 1e8,
                    hash_type_str,
                    hex::encode(out.lock_script.code_hash),
                    hex::encode(out.lock_script.args),
                    type_str,
                );
            }

            println!("cell_deps ({}):", tx.cell_deps.len());
            for (i, dep) in tx.cell_deps.iter().enumerate() {
                let dep_type_str = if dep.dep_type == 1 {
                    "dep_group"
                } else {
                    "code"
                };
                println!(
                    "  [{}]  0x{}:{}  {}",
                    i,
                    hex::encode(dep.outpoint.tx_hash),
                    dep.outpoint.index,
                    dep_type_str
                );
            }

            println!("witnesses ({}):", tx.witnesses.len());
            for (i, w) in tx.witnesses.iter().enumerate() {
                match &w.lock {
                    Some(sig) if !sig.is_empty() => {
                        println!("  [{}]  0x{}  ({} bytes)", i, hex::encode(sig), sig.len())
                    }
                    _ => println!("  [{}]  0x  (empty)", i),
                }
            }

            println!("tx hash:   0x{}", hex::encode(tx.rpc_raw_tx_hash()));
        }

        TxCmd::Hash(args) => {
            let json = std::fs::read_to_string(&args.tx)?;
            let tx: CKBTransaction = serde_json::from_str(&json)?;
            println!("0x{}", hex::encode(tx.rpc_raw_tx_hash()));
        }

        TxCmd::Status(args) => {
            let cfg = load_config();
            let rpc_url = active_rpc(&cfg, args.rpc.as_deref(), devnet_flag);
            let rpc = CkbRpcClient::new(&rpc_url);

            let hash = parse32(&args.tx_hash)?;
            let result = rpc.get_transaction(hash)?;

            let tx_status = &result["tx_status"];
            let status = tx_status["status"].as_str().unwrap_or("unknown");
            let block_hash = tx_status["block_hash"]
                .as_str()
                .unwrap_or("(not yet committed)");
            let block_number = tx_status["block_number"]
                .as_str()
                .map(|s| {
                    u64::from_str_radix(s.trim_start_matches("0x"), 16)
                        .map(|n| n.to_string())
                        .unwrap_or_else(|_| s.to_string())
                })
                .unwrap_or_else(|| "(not yet committed)".to_string());

            println!("status:       {}", status);
            println!("block_hash:   {}", block_hash);
            println!("block_number: {}", block_number);

            if status == "rejected" {
                let reason = tx_status["reason"].as_str().unwrap_or("unknown");
                println!("reason:       {}", reason);
            }
        }

        TxCmd::Send(args) => {
            let cfg = load_config();
            let rpc_url = active_rpc(&cfg, args.rpc.as_deref(), devnet_flag);
            let net = active_network(&cfg, devnet_flag);
            let secret = resolve_secret(args.keypair.as_deref(), args.secret.as_deref())?;
            let account = Account::from_secret(secret);
            let sk = secp256k1::SecretKey::from_byte_array(secret)?;
            let to_pubkey_hash = parse_addr_or_pubkey_hash(&args.to)?;
            let amount = parse_ckb_amount(&args.amount)?;

            let rpc = CkbRpcClient::new(&rpc_url);

            let cells = rpc.get_live_cells(account.pubkey_hash)?;
            if cells.is_empty() {
                return Err(anyhow!(
                    "no live cells found for this account on {}",
                    rpc_url
                ));
            }

            // Use a pessimistic fee estimate (3 inputs, 2 outputs) for initial coin selection,
            // then recompute with the actual input count.
            let fee_for_selection = if args.fee > 0 {
                args.fee
            } else {
                estimate_fee(3, 2)
            };
            let (selected_indices, _) = select_cells(&cells, amount, fee_for_selection)?;

            let n_inputs = selected_indices.len();
            let total_in: u64 = selected_indices.iter().map(|&i| cells[i].capacity).sum();
            let actual_fee = if args.fee > 0 {
                args.fee
            } else {
                estimate_fee(n_inputs, 2)
            };

            let inputs: Vec<CellInput> = selected_indices
                .iter()
                .map(|&i| CellInput {
                    previous_outpoint: cells[i].out_point,
                    since: 0,
                })
                .collect();

            let raw_change = total_in.saturating_sub(amount).saturating_sub(actual_fee);
            let change = if raw_change > 0 && raw_change < MIN_CELL_CAPACITY {
                0
            } else {
                raw_change
            };
            let actual_fee = if change == 0 {
                total_in - amount
            } else {
                actual_fee
            };

            let sender_lock = CkbScript {
                code_hash: SECP256K1_CODE_HASH,
                hash_type: 1,
                args: account.pubkey_hash,
            };
            let recipient_lock = CkbScript {
                code_hash: SECP256K1_CODE_HASH,
                hash_type: 1,
                args: to_pubkey_hash,
            };

            let mut outputs = vec![CellOutput {
                capacity: amount,
                lock_script: recipient_lock,
                type_script: None,
            }];
            if change > 0 {
                outputs.push(CellOutput {
                    capacity: change,
                    lock_script: sender_lock,
                    type_script: None,
                });
            }

            let n_inputs = inputs.len();
            let empty_witnesses: Vec<WitnessArgs> = (0..n_inputs)
                .map(|_| WitnessArgs {
                    lock: None,
                    input_type: None,
                    output_type: None,
                })
                .collect();

            let cell_deps = vec![CellDep {
                outpoint: OutPoint {
                    tx_hash: net.secp256k1_dep_tx_hash(),
                    index: SECP256K1_DEP_INDEX,
                },
                dep_type: SECP256K1_DEP_TYPE,
            }];

            let mut tx = CKBTransaction {
                version: 0,
                cell_deps,
                header_deps: [0u8; 32],
                inputs,
                witnesses: empty_witnesses,
                outputs,
                output_data: vec![],
            };

            let signature = tx.create_rpc_signature(sk);
            tx.witnesses[0].lock = Some(signature.to_vec());

            let tx_json = tx.to_rpc_value();
            let tx_hash = rpc.send_transaction(tx_json)?;

            println!("network:  {}", net);
            println!(
                "sent:     {:.8} CKB ({} shannons) to 0x{}",
                amount as f64 / 1e8,
                amount,
                hex::encode(to_pubkey_hash)
            );
            if change > 0 {
                println!(
                    "change:   {:.8} CKB ({} shannons) back to self",
                    change as f64 / 1e8,
                    change
                );
            }
            println!(
                "fee:      {} shannons ({:.8} CKB)",
                actual_fee,
                actual_fee as f64 / 1e8
            );
            println!("tx hash:  0x{}", hex::encode(tx_hash));
            println!(
                "explorer: {}/transaction/0x{}",
                net.explorer_base(),
                hex::encode(tx_hash)
            );
        }
    }
    Ok(())
}

fn cmd_cell(args: CellArgs, devnet_flag: bool) -> Result<()> {
    let cfg = load_config();
    let rpc_url = active_rpc(&cfg, args.rpc.as_deref(), devnet_flag);
    let net = active_network(&cfg, devnet_flag);
    let rpc = CkbRpcClient::new(&rpc_url);

    let (tx_hash, index) = parse_outpoint_str(&args.outpoint)?;
    let out_point = crate::network::transaction::OutPoint { tx_hash, index };
    let cell = rpc.get_cell(&out_point, args.data)?;

    let output = &cell["output"];
    let cap_hex = output["capacity"].as_str().unwrap_or("0x0");
    let capacity = u64::from_str_radix(cap_hex.trim_start_matches("0x"), 16)
        .map_err(|_| anyhow!("could not parse capacity '{}'", cap_hex))?;

    println!("network:    {}", net);
    println!("outpoint:   0x{}:{}", hex::encode(tx_hash), index);
    println!(
        "capacity:   {:.8} CKB  ({} shannons)",
        capacity as f64 / 1e8,
        capacity
    );

    let lock = &output["lock"];
    println!("lock:");
    println!(
        "  code_hash:  {}",
        lock["code_hash"].as_str().unwrap_or("?")
    );
    println!(
        "  hash_type:  {}",
        lock["hash_type"].as_str().unwrap_or("?")
    );
    println!("  args:       {}", lock["args"].as_str().unwrap_or("?"));

    match output.get("type") {
        Some(t) if !t.is_null() => {
            println!("type:");
            println!("  code_hash:  {}", t["code_hash"].as_str().unwrap_or("?"));
            println!("  hash_type:  {}", t["hash_type"].as_str().unwrap_or("?"));
            println!("  args:       {}", t["args"].as_str().unwrap_or("?"));
        }
        _ => println!("type:       (none)"),
    }

    if args.data {
        let data = cell["data"]["content"].as_str().unwrap_or("0x");
        if data == "0x" || data.is_empty() {
            println!("data:       0x  (empty)");
        } else {
            println!("data:       {}", data);
        }
    }

    Ok(())
}

fn cmd_tip(args: TipArgs, devnet_flag: bool) -> Result<()> {
    let cfg = load_config();
    let rpc_url = active_rpc(&cfg, args.rpc.as_deref(), devnet_flag);
    let net = active_network(&cfg, devnet_flag);
    let rpc = CkbRpcClient::new(&rpc_url);

    let header = rpc.get_tip_header()?;

    let number_hex = header["number"].as_str().unwrap_or("0x0");
    let number = u64::from_str_radix(number_hex.trim_start_matches("0x"), 16)
        .map_err(|_| anyhow!("could not parse block number"))?;
    let hash = header["hash"].as_str().unwrap_or("?");
    let timestamp = header["timestamp"].as_str().unwrap_or("?");

    println!("network:    {}", net);
    println!("block:      {}", number);
    println!("hash:       {}", hash);
    println!("timestamp:  {}  (unix ms)", timestamp);

    Ok(())
}

fn cmd_block(args: BlockArgs, devnet_flag: bool) -> Result<()> {
    let cfg = load_config();
    let rpc_url = active_rpc(&cfg, args.rpc.as_deref(), devnet_flag);
    let net = active_network(&cfg, devnet_flag);
    let rpc = CkbRpcClient::new(&rpc_url);

    let block = if args.target.starts_with("0x") && args.target.len() == 66 {
        let hash = parse32(&args.target)?;
        rpc.get_block(hash)?
    } else {
        let n: u64 = args.target.parse().map_err(|_| {
            anyhow!(
                "invalid block target '{}' — use a decimal number or 0x-prefixed 32-byte hash",
                args.target
            )
        })?;
        rpc.get_block_by_number(n)?
    };

    let header = &block["header"];
    let number_hex = header["number"].as_str().unwrap_or("0x0");
    let number = u64::from_str_radix(number_hex.trim_start_matches("0x"), 16)
        .map_err(|_| anyhow!("could not parse block number"))?;
    let hash = header["hash"].as_str().unwrap_or("?");
    let timestamp = header["timestamp"].as_str().unwrap_or("?");
    let tx_count = block["transactions"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);

    println!("network:    {}", net);
    println!("number:     {}", number);
    println!("hash:       {}", hash);
    println!("timestamp:  {}  (unix ms)", timestamp);
    println!("txs:        {}", tx_count);
    println!("explorer:   {}/block/{}", net.explorer_base(), hash);

    Ok(())
}

fn cmd_rpc(args: RpcArgs, devnet_flag: bool) -> Result<()> {
    let cfg = load_config();
    let rpc_url = active_rpc(&cfg, args.rpc.as_deref(), devnet_flag);
    let rpc = CkbRpcClient::new(&rpc_url);

    let params: serde_json::Value = match args.params {
        Some(ref s) => serde_json::from_str(s)
            .map_err(|e| anyhow!("params must be a valid JSON array: {}", e))?,
        None => serde_json::json!([]),
    };

    if !params.is_array() {
        return Err(anyhow!(
            "params must be a JSON array, e.g. '[]' or '[\"0x1\"]'"
        ));
    }

    let result = rpc.call(&args.method, params)?;
    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}
