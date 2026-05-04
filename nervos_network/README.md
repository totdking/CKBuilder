# Week 1 (27/04/2026 -> 4/05/2026)

---

## nervos_network

A mock implementation of the CKB (Nervos Network) blockchain in Rust, built from first principles to understand the Cell model, transaction lifecycle, and cryptographic primitives used by CKB.

### What was built

**Cell model** (`src/data/`)
- `CkbScript` — lock and type script representation with code_hash, hash_type, and args
- `CkbCell` — a cell with capacity, data, lock script, and type script; ownership checked via `can_unlock_script`
- `Account` — keypair derived from a 32-byte secret: secp256k1 private key → compressed pubkey → Blake2b-160 pubkey hash
- `Address` — full CKB bech32m address encoding from a lock script

**Network layer** (`src/network/`)
- `CKBTransaction` — full transaction structure (version, cell_deps, header_deps, inputs, outputs, witnesses, output_data)
- `hash()` — Blake2b personalized hash of the serialized raw transaction (excluding witnesses)
- `create_sighash()` — CKB sighash_all algorithm: `blake2b(raw_tx_hash || len || witness[0]_zeroed_lock || ...)`
- `create_signature()` — 65-byte recoverable secp256k1 ECDSA signature `[r(32) | s(32) | v(1)]`
- `sign()` — ownership check + sign + embed signature into `witnesses[0].lock`
- `validate_spend()` — structural check: since == 0, witness slot exists, lock is filled
- `MockLedger` — in-memory UTXO set backed by a HashMap; `birth_cell` / `kill_cell` / `is_live`; persists to JSON

**Serialization**
- All transaction and cell structures serialize to/from JSON via serde, with byte arrays encoded as hex strings
- Molecule serialization for the CKB binary wire format (generated from `.mol` schema files)

---

## CLI — ckbuilder

An interactive command-line tool for working with the mock CKB network.

### Setup

**Requirements:** Rust toolchain installed ([rustup.rs](https://rustup.rs))

```sh
# Clone and enter the project
git clone <repo-url>
cd nervos_network

# Install the ckb binary globally (builds release + puts it on PATH)
cargo install --path .
```

After that, `ckb` is available from any directory in your terminal. No need to prefix with `./target/...`.

```sh
ckb --help
```

To update the binary after making code changes, re-run `cargo install --path .`.

> **Note:** All commands read and write the ledger at `src/ledger/ledger.json` relative to your working directory. Run `ckb` commands from the project root so the ledger path resolves correctly. You can override it on any command with `--ledger <path>`.

The ledger state persists across invocations in `src/ledger/ledger.json` (override with `--ledger <path>`).

---

### account — generate a keypair

```sh
# Generate a random account
ckb account

# Derive from a known secret
ckb account --secret <32-byte-hex>
```

Output:
```
secret_key:  <hex>
pubkey_hash: <hex>
```

---

### address — derive a bech32m CKB address

```sh
ckb address \
  --code-hash <32-byte-hex> \
  --hash-type <0|1|2|4> \
  --args <20-byte-pubkey-hash-hex>
```

`hash_type`: `0`=Data, `1`=Type, `2`=Data1, `4`=Data2

Example (mainnet secp256k1 lock):
```sh
ckb address \
  --code-hash 9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8 \
  --hash-type 1 \
  --args b39bbc0b3673c7d36450bc14cfcdad2d559c6c64
```

---

### ledger — manage the persistent UTXO set

**Add a cell:**
```sh
ckb ledger birth \
  --tx-hash <32-byte-hex> \
  --index <u32> \
  --capacity <shannons> \
  --lock-code-hash <32-byte-hex> \
  --lock-hash-type <u8> \
  --lock-args <20-byte-hex>
```

**Spend a cell:**
```sh
ckb ledger kill --tx-hash <hex> --index <u32>
```

**Check if a cell is live:**
```sh
ckb ledger status --tx-hash <hex> --index <u32>
```

**List all live cells:**
```sh
ckb ledger list
```

---

### tx — build, sign, and inspect transactions

**Build an unsigned transaction:**
```sh
ckb tx build \
  --from-tx-hash <hex> \
  --from-index <u32> \
  --to-capacity <shannons> \
  --to-code-hash <hex> \
  --to-hash-type <u8> \
  --to-args <20-byte-hex> \
  --out tx.json
```

Writes an unsigned transaction to `src/ledger/txs/tx.json` and prints the tx hash.

**Sign a transaction:**
```sh
ckb tx sign --tx tx.json --secret <32-byte-hex>
```

Embeds a 65-byte recoverable signature into `witnesses[0].lock` and overwrites the file.

**Get the transaction hash:**
```sh
ckb tx hash --tx tx.json
```

**Validate a transaction input:**
```sh
ckb tx validate --tx tx.json --input-index 0
```

Checks: `since == 0`, witness slot exists, and `witnesses[input_index].lock` is filled.

---

### Full example — Alice sends CKB to Bob

```sh
# 1. Generate Alice's account
ckb account --secret 0101010101010101010101010101010101010101010101010101010101010101
# pubkey_hash: <alice_pubkey_hash>

# 2. Birth Alice's cell into the ledger
ckb ledger birth \
  --tx-hash aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
  --index 0 \
  --capacity 10000000000 \
  --lock-code-hash 9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8 \
  --lock-hash-type 1 \
  --lock-args <alice_pubkey_hash>

# 3. Build a transfer to Bob (identified by his pubkey hash)
ckb tx build \
  --from-tx-hash aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
  --from-index 0 \
  --to-capacity 9900000000 \
  --to-code-hash 9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8 \
  --to-hash-type 1 \
  --to-args <bob_pubkey_hash> \
  --out transfer.json

# 4. Alice signs
ckb tx sign --tx transfer.json --secret 0101010101010101010101010101010101010101010101010101010101010101

# 5. Validate
ckb tx validate --tx transfer.json
# valid

# 6. Update the ledger: kill Alice's cell, birth Bob's
ckb ledger kill \
  --tx-hash aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
  --index 0

ckb ledger birth \
  --tx-hash <tx_hash_from_step_3> \
  --index 0 \
  --capacity 9900000000 \
  --lock-code-hash 9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8 \
  --lock-hash-type 1 \
  --lock-args <bob_pubkey_hash>
```
