# network

The network layer handles everything that requires talking to a CKB node or serializing data for the wire format: transaction construction, signing, fee estimation, RPC calls, and coin selection.

---

## rpc.rs

### Network enum

Represents the three supported networks. Each variant carries its own RPC URL, bech32 HRP, block explorer base URL, and — critically — its own secp256k1 dep_group transaction hash.

| Variant | RPC | Address prefix |
|---------|-----|----------------|
| `Testnet` | https://testnet.ckb.dev | `ckt1...` |
| `Mainnet` | https://mainnet.ckb.dev | `ckb1...` |
| `Devnet` | http://localhost:8114 | `ckt1...` |

The dep_group tx hash is **network-specific** because it comes from each network's genesis block. Using the wrong hash causes `TransactionFailedToResolve` errors. Three constants are defined:

```rust
SECP256K1_DEP_TX_HASH_MAINNET  // genesis tx[1] on mainnet
SECP256K1_DEP_TX_HASH_TESTNET  // genesis tx[1] on Pudge testnet
SECP256K1_DEP_TX_HASH_DEVNET   // genesis tx[1] on offckb devnet
```

`Network::secp256k1_dep_tx_hash()` returns the correct one for the active network.

The `SECP256K1_CODE_HASH` (the hash of the secp256k1 binary itself) is the **same on all networks** — it is derived from the binary, not the genesis block.

### CkbRpcClient

A blocking JSON-RPC 2.0 HTTP client. All methods POST to the node's RPC endpoint.

| Method | RPC call | Description |
|--------|----------|-------------|
| `get_live_cells(pubkey_hash)` | `get_cells` | Fetch all live cells owned by a pubkey_hash via the indexer |
| `get_cell_info(outpoint)` | `get_live_cell` | Fetch the capacity and lock args of a specific cell by outpoint |
| `send_transaction(tx_json)` | `send_transaction` | Broadcast a signed transaction; returns the tx hash |
| `get_tip_block_number()` | `get_tip_block_number` | Verify the node is reachable; returns current chain height |

`get_live_cells` paginates automatically using the `last_cursor` field, so it handles accounts with more than 100 cells.

### Fee estimation

`estimate_fee(n_inputs, n_outputs) -> u64` calculates a fee based on the approximate serialized transaction size in bytes, multiplied by CKB's minimum fee rate of 1000 shannons/KB.

```
size ≈ 200 (base)
     + n_inputs  × 48   (per input)
     + n_outputs × 97   (per output, secp256k1 lock)
     + 93               (first witness with 65-byte sig)
     + (n_inputs - 1) × 16   (empty witnesses for additional inputs)

fee = ceil(size × 1000 / 1024)
    clamped to [1000, 100_000] shannons
```

This replaces the old hardcoded 1000-shannon constant and scales correctly with multi-input transactions.

### Coin selection

`select_cells(cells, amount, fee) -> (selected_indices, change)` implements a greedy coin selection strategy:

1. Cells are pre-sorted by capacity descending (largest first)
2. Accumulate cells until the total covers `amount + fee`
3. If the leftover change is below `MIN_CELL_CAPACITY` (61 CKB), it is absorbed into the miner fee rather than creating an invalid dust cell

---

## transaction.rs

### CKBTransaction

The top-level transaction struct. Serializes to/from JSON for storage and to the Molecule binary format for hashing and broadcasting.

| Field | Type | Description |
|---|---|---|
| `version` | `u32` | Transaction version (always 0) |
| `cell_deps` | `Vec<CellDep>` | Read-only cell references (e.g. the secp256k1 script binary) |
| `header_deps` | `[u8; 32]` | Block header dependency (zeroed for standard transfers) |
| `inputs` | `Vec<CellInput>` | Cells being consumed; each has an outpoint and a `since` lock |
| `witnesses` | `Vec<WitnessArgs>` | One slot per input; `witnesses[0].lock` holds the 65-byte signature |
| `outputs` | `Vec<CellOutput>` | Cells being created |
| `output_data` | `Vec<u8>` | Data attached to outputs (empty for simple transfers) |

### Key methods

**`rpc_raw_tx_hash() -> [u8; 32]`**

Computes the transaction hash the way the real CKB node does it — using a Molecule `Byte32Vec` (not `Byte32`) for `header_deps`. This distinction matters: the auto-generated schema uses the wrong type, so this method assembles the correct layout manually to produce a hash that matches the node.

**`create_rpc_sighash() -> [u8; 32]`**

Implements CKB's `sighash_all` algorithm over the real-network tx hash:

```
blake2b(
    rpc_raw_tx_hash
    || len(witnesses[0]) || witnesses[0] with lock zeroed to 65 bytes
    || len(witnesses[1]) || witnesses[1]
    ...
)
```

`witnesses[0].lock` is replaced with 65 zero bytes before hashing so the digest can be computed before the signature exists.

**`create_rpc_signature(sk) -> [u8; 65]`**

Signs the sighash with a secp256k1 recoverable ECDSA signature. Returns 65 bytes: `[r(32) | s(32) | v(1)]`. The recovery byte `v` lets a verifier reconstruct the signer's public key without it being transmitted separately.

**`to_rpc_value() -> serde_json::Value`**

Serializes the transaction into the exact JSON format the CKB node's `send_transaction` RPC expects — all numbers as `0x`-prefixed hex strings, `dep_type` as `"dep_group"` or `"code"`, etc.

**`validate_spend(input_index, cells) -> Result<()>`**

Full cryptographic validation of a single input (used in tests):
1. `since == 0` — no time lock
2. Witness slot exists and has 65 bytes
3. Recovers the signer's pubkey hash from the signature
4. Confirms it matches `cells[input_index].lock_script.args`

---

## consensus.rs

### MockLedger

An in-memory UTXO set backed by `HashMap<OutPoint, CellOutput>`. Used only in tests — not in production.

| Method | Description |
|--------|-------------|
| `birth_cell(outpoint, cell)` | Add a live cell; rejects duplicate outpoints |
| `kill_cell(outpoint)` | Remove a live cell; rejects non-existent outpoints |
| `is_live(outpoint)` | Check if an outpoint is in the live set |
| `load(path)` | Deserialize from JSON; returns empty ledger if file absent |
| `save(path)` | Serialize to JSON; creates parent directories if needed |

The test suite in `transaction.rs` uses `MockLedger` to run full end-to-end scenarios (Alice sends to Bob, double-spend detection, multi-cell merges) without a running node.

---

## block.rs

Block header structure. Currently a stub — the CLI does not need to construct or parse blocks directly, only reference them via header_deps.

---

## Transaction lifecycle on a real network

```
1. ckb balance --utxos               → find a cell to spend (get_live_cells)
2. ckb tx build --from ... --to ...  → construct unsigned tx (fetch cell via get_live_cell)
3. ckb tx sign --tx tx.json          → sign (create_rpc_sighash → create_rpc_signature)
4. ckb tx broadcast --tx tx.json     → submit (to_rpc_value → send_transaction)
5. node validates and mines           → cell state changes on-chain
```
