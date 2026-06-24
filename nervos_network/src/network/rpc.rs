use anyhow::{Result, anyhow};
use hex_literal::hex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::cmp::Reverse;
use std::fmt;
use std::time::Duration;

use crate::network::transaction::OutPoint;

// ── Testnet system script constants ──────────────────────────────────────────

/// secp256k1/blake160 lock code_hash (hash_type = type) — same on all networks
pub const SECP256K1_CODE_HASH: [u8; 32] =
    hex!("9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");

/// dep_group (bundles secp256k1 lib + sighash_all) — MAINNET genesis tx[1], out[0]
pub const SECP256K1_DEP_TX_HASH_MAINNET: [u8; 32] =
    hex!("71a7ba8fc96349fea0ed3a5c47992e3b4084b031a42264a018e0072e8172e46c");

/// dep_group (bundles secp256k1 lib + sighash_all) — PUDGE TESTNET genesis tx[1], out[0]
pub const SECP256K1_DEP_TX_HASH_TESTNET: [u8; 32] =
    hex!("f8de3bb47d055cdf460d93a2a6e1b05f7432f9777c8c474abf4eec1d4aee5d37");

/// dep_group (bundles secp256k1 lib + sighash_all) — offckb DEVNET genesis tx[1], out[0]
pub const SECP256K1_DEP_TX_HASH_DEVNET: [u8; 32] =
    hex!("4d804f1495612631da202fe9902fa9899118554b08138cfe5dfb50e1ede76293");

pub const SECP256K1_DEP_INDEX: u32 = 0;
pub const SECP256K1_DEP_TYPE: u8 = 1; // dep_group

/// Minimum cell capacity: 61 CKB in shannons (a cell must cover its own storage cost)
pub const MIN_CELL_CAPACITY: u64 = 6_100_000_000;

/// Fallback fee floor in shannons (0.00001 CKB). Real fee is computed by `estimate_fee`.
pub const DEFAULT_FEE: u64 = 1_000;

/// Maximum fee we will ever spend (0.001 CKB = 100_000 shannons) — safety cap.
pub const MAX_FEE: u64 = 100_000;

/// CKB minimum fee rate: 1000 shannons per KB.
pub const FEE_RATE_SHANNONS_PER_KB: u64 = 1_000;

/// Estimates the transaction fee in shannons based on the number of inputs and outputs.
///
/// Uses a byte-size model for standard secp256k1 P2PKH transactions:
/// - Base (version + 1 cell_dep + header_deps + table overhead): ~200 bytes
/// - Per input: 48 bytes (outpoint 36 + since 8 + molecule overhead 4)
/// - Per output: 97 bytes (capacity 8 + secp256k1 lock script 69 + type_opt 4 + overhead 16)
/// - First witness (65-byte sig packed): ~93 bytes
/// - Each additional empty witness: ~16 bytes
///
/// Fee = ceil(size × FEE_RATE / 1024), clamped to [DEFAULT_FEE, MAX_FEE].
pub fn estimate_fee(n_inputs: usize, n_outputs: usize) -> u64 {
    let size = 200_usize
        + n_inputs * 48
        + n_outputs * 97
        + 93                                            // first witness
        + n_inputs.saturating_sub(1) * 16; // additional empty witnesses
    let fee = (size as u64 * FEE_RATE_SHANNONS_PER_KB).div_ceil(1_024);
    fee.clamp(DEFAULT_FEE, MAX_FEE)
}

pub const TESTNET_RPC: &str = "https://testnet.ckb.dev";
pub const MAINNET_RPC: &str = "https://mainnet.ckb.dev";
pub const DEVNET_RPC: &str = "http://localhost:8114";

// ── Network ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Testnet,
    Mainnet,
    /// Local offckb devnet (http://localhost:8114)
    Devnet,
}

impl Network {
    /// bech32 human-readable part for addresses on this network
    pub fn hrp(self) -> &'static str {
        match self {
            Network::Testnet | Network::Devnet => "ckt",
            Network::Mainnet => "ckb",
        }
    }

    /// Default RPC node URL for this network
    pub fn rpc_url(self) -> &'static str {
        match self {
            Network::Testnet => TESTNET_RPC,
            Network::Mainnet => MAINNET_RPC,
            Network::Devnet => DEVNET_RPC,
        }
    }

    /// dep_group tx hash for the secp256k1/blake160-sighash-all system script on this network
    pub fn secp256k1_dep_tx_hash(self) -> [u8; 32] {
        match self {
            Network::Mainnet => SECP256K1_DEP_TX_HASH_MAINNET,
            Network::Testnet => SECP256K1_DEP_TX_HASH_TESTNET,
            Network::Devnet => SECP256K1_DEP_TX_HASH_DEVNET,
        }
    }

    /// Block explorer base URL for this network
    pub fn explorer_base(self) -> &'static str {
        match self {
            Network::Testnet => "https://pudge.explorer.nervos.org",
            Network::Mainnet => "https://explorer.nervos.org",
            Network::Devnet => "http://localhost:3000",
        }
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Network::Testnet => write!(f, "testnet"),
            Network::Mainnet => write!(f, "mainnet"),
            Network::Devnet => write!(f, "devnet"),
        }
    }
}

// ── LiveCell ─────────────────────────────────────────────────────────────────

/// A live (unspent) cell returned by the indexer RPC
#[derive(Debug)]
pub struct LiveCell {
    pub out_point: OutPoint,
    pub capacity: u64,
}

// ── CkbRpcClient ─────────────────────────────────────────────────────────────

pub struct CkbRpcClient {
    url: String,
    client: Client,
}

impl CkbRpcClient {
    pub fn new(url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self {
            url: url.to_string(),
            client,
        }
    }

    /// Verifies the node is reachable and returns the current tip block number.
    #[allow(dead_code)]
    pub fn get_tip_block_number(&self) -> Result<u64> {
        let result = self.rpc("get_tip_block_number", json!([]))?;
        let hex_str = result
            .as_str()
            .ok_or_else(|| anyhow!("expected hex string"))?;
        let n = u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)?;
        Ok(n)
    }

    /// Fetches the capacity and lock args of a specific cell by outpoint.
    /// Used by `tx build` to calculate change automatically.
    pub fn get_cell_info(&self, out_point: &OutPoint) -> Result<(u64, [u8; 20])> {
        let params = json!([{
            "tx_hash": format!("0x{}", hex::encode(out_point.tx_hash)),
            "index":   format!("0x{:x}", out_point.index),
        }, false]);

        let result = self.rpc("get_live_cell", params)?;

        let status = result["status"].as_str().unwrap_or("unknown");
        if status != "live" {
            return Err(anyhow!(
                "cell {}:{} is not live (status: {})",
                hex::encode(out_point.tx_hash),
                out_point.index,
                status
            ));
        }

        let output = &result["cell"]["output"];

        let cap_hex = output["capacity"]
            .as_str()
            .ok_or_else(|| anyhow!("missing capacity in cell response"))?;
        let capacity = u64::from_str_radix(cap_hex.trim_start_matches("0x"), 16)?;

        let args_hex = output["lock"]["args"]
            .as_str()
            .ok_or_else(|| anyhow!("missing lock args in cell response"))?
            .trim_start_matches("0x");
        let args_bytes = hex::decode(args_hex)?;
        let lock_args: [u8; 20] = args_bytes.try_into().map_err(|_| {
            anyhow!("lock args are not 20 bytes — only secp256k1/blake160 cells are supported")
        })?;

        Ok((capacity, lock_args))
    }

    /// Fetches all live cells owned by `pubkey_hash` using the secp256k1 lock.
    /// Returns cells sorted by capacity descending (largest first, for greedy coin selection).
    pub fn get_live_cells(&self, pubkey_hash: [u8; 20]) -> Result<Vec<LiveCell>> {
        let search_key = json!({
            "script": {
                "code_hash": format!("0x{}", hex::encode(SECP256K1_CODE_HASH)),
                "hash_type": "type",
                "args": format!("0x{}", hex::encode(pubkey_hash)),
            },
            "script_type": "lock",
            "with_data": false,
        });

        let mut cells: Vec<LiveCell> = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let params = match &cursor {
                None => json!([search_key, "asc", "0x64"]),
                Some(c) => json!([search_key, "asc", "0x64", c]),
            };

            let result = self.rpc("get_cells", params)?;
            let objects = result["objects"]
                .as_array()
                .ok_or_else(|| anyhow!("expected objects array in get_cells response"))?;

            for obj in objects {
                let cap_hex = obj["output"]["capacity"]
                    .as_str()
                    .ok_or_else(|| anyhow!("missing capacity"))?;
                let capacity = u64::from_str_radix(cap_hex.trim_start_matches("0x"), 16)?;

                let tx_hash_hex = obj["out_point"]["tx_hash"]
                    .as_str()
                    .ok_or_else(|| anyhow!("missing tx_hash"))?
                    .trim_start_matches("0x");
                let tx_hash_bytes = hex::decode(tx_hash_hex)?;
                let mut tx_hash = [0u8; 32];
                tx_hash.copy_from_slice(&tx_hash_bytes);

                let index_hex = obj["out_point"]["index"]
                    .as_str()
                    .ok_or_else(|| anyhow!("missing index"))?;
                let index = u32::from_str_radix(index_hex.trim_start_matches("0x"), 16)?;

                cells.push(LiveCell {
                    out_point: OutPoint { tx_hash, index },
                    capacity,
                });
            }

            let next_cursor = result["last_cursor"].as_str().unwrap_or("");
            if objects.len() < 100 || next_cursor.is_empty() || next_cursor == "0x" {
                break;
            }
            cursor = Some(next_cursor.to_string());
        }

        cells.sort_by_key(|b| Reverse(b.capacity));
        Ok(cells)
    }

    /// Broadcasts a signed transaction to the CKB node.
    /// Returns the transaction hash (32 bytes).
    pub fn send_transaction(&self, tx_json: Value) -> Result<[u8; 32]> {
        let result = self.rpc("send_transaction", json!([tx_json, "passthrough"]))?;
        let hash_hex = result
            .as_str()
            .ok_or_else(|| anyhow!("expected hex tx hash from send_transaction"))?
            .trim_start_matches("0x");
        let bytes = hex::decode(hash_hex)?;
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Ok(hash)
    }

    /// Generic JSON-RPC 2.0 POST helper. Returns the `result` field or errors on `error`.
    fn rpc(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let resp: Value = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .map_err(|e| anyhow!("RPC request to {} failed: {}", self.url, e))?
            .json()
            .map_err(|e| anyhow!("failed to parse RPC response: {}", e))?;

        if let Some(err) = resp.get("error") {
            return Err(anyhow!("RPC error: {}", err));
        }

        resp.get("result")
            .cloned()
            .ok_or_else(|| anyhow!("RPC response missing 'result' field"))
    }
}

// ── Coin selection ────────────────────────────────────────────────────────────

/// Greedy coin selection: picks the fewest cells (largest first) to cover `amount + fee`.
///
/// Returns `(selected_indices, change_amount)`.
/// If change would be below `MIN_CELL_CAPACITY`, it is absorbed into the fee instead.
/// Errors if the total balance is insufficient.
pub fn select_cells(cells: &[LiveCell], amount: u64, fee: u64) -> Result<(Vec<usize>, u64)> {
    let need = amount
        .checked_add(fee)
        .ok_or_else(|| anyhow!("amount + fee overflow"))?;
    let total: u64 = cells.iter().map(|c| c.capacity).sum();
    if total < need {
        return Err(anyhow!(
            "insufficient balance: have {} shannons ({:.2} CKB), need {} shannons ({:.2} CKB)",
            total,
            total as f64 / 1e8,
            need,
            need as f64 / 1e8,
        ));
    }

    let mut selected = Vec::new();
    let mut sum = 0u64;
    for (i, cell) in cells.iter().enumerate() {
        selected.push(i);
        sum += cell.capacity;
        if sum >= need {
            break;
        }
    }

    let raw_change = sum - need;
    // If change is too small to form a valid cell, absorb it into the fee
    let change = if raw_change > 0 && raw_change < MIN_CELL_CAPACITY {
        0
    } else {
        raw_change
    };

    Ok((selected, change))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(capacities: &[u64]) -> Vec<LiveCell> {
        capacities
            .iter()
            .map(|&c| LiveCell {
                out_point: OutPoint {
                    tx_hash: [0u8; 32],
                    index: 0,
                },
                capacity: c,
            })
            .collect()
    }

    #[test]
    fn select_exact_amount() {
        let cs = cells(&[10_000_000_000]);
        let (indices, change) = select_cells(&cs, 9_999_999_000, 1_000).unwrap();
        assert_eq!(indices, vec![0]);
        assert_eq!(change, 0);
    }

    #[test]
    fn select_with_change() {
        // 200 CKB available, send 100, fee 1000 → change = 100 CKB - 1000 shannons
        let cs = cells(&[20_000_000_000]);
        let (indices, change) = select_cells(&cs, 10_000_000_000, DEFAULT_FEE).unwrap();
        assert_eq!(indices, vec![0]);
        assert_eq!(change, 20_000_000_000 - 10_000_000_000 - DEFAULT_FEE);
    }

    #[test]
    fn select_multiple_cells() {
        // Needs two cells to cover the amount
        let cs = cells(&[6_100_000_000, 6_100_000_000]);
        let (indices, change) = select_cells(&cs, 10_000_000_000, DEFAULT_FEE).unwrap();
        assert_eq!(indices.len(), 2);
        let total: u64 = indices.iter().map(|&i| cs[i].capacity).sum();
        assert!(total >= 10_000_000_000 + DEFAULT_FEE);
        assert!(change == 0 || change >= MIN_CELL_CAPACITY);
    }

    #[test]
    fn select_small_change_absorbed_into_fee() {
        // raw_change = 1000 shannons (< MIN_CELL_CAPACITY = 6.1 CKB) → absorbed into fee
        let cs = cells(&[10_000_002_000]);
        let (_, change) = select_cells(&cs, 10_000_001_000, 0).unwrap();
        assert_eq!(change, 0);
    }

    #[test]
    fn select_insufficient_balance() {
        let cs = cells(&[1_000_000]);
        assert!(select_cells(&cs, 10_000_000_000, DEFAULT_FEE).is_err());
    }

    #[test]
    fn select_empty_cells() {
        let cs: Vec<LiveCell> = vec![];
        assert!(select_cells(&cs, 1, 0).is_err());
    }
}
