# network

The network layer models how CKB transactions are constructed, signed, validated, and how cell state is tracked across the mock ledger.

---

## transaction.rs

### Structs

**`CKBTransaction`** — the top-level transaction object.

| Field | Type | Description |
|---|---|---|
| `version` | `u32` | Transaction version (currently 0) |
| `cell_deps` | `Vec<CellDep>` | Cells referenced but not consumed (e.g. lock script binaries) |
| `header_deps` | `[u8;32]` | Block header dependency (unused in mock, zeroed) |
| `inputs` | `Vec<CellInput>` | Cells being consumed |
| `witnesses` | `Vec<WitnessArgs>` | Proof data — one slot per input; `witnesses[0].lock` holds the signature |
| `outputs` | `Vec<CellOutput>` | Cells being created |
| `output_data` | `Vec<u8>` | Raw data attached to outputs |

**`CellInput`** — a reference to a live cell being spent.
- `previous_outpoint` — identifies the cell by tx_hash + index
- `since` — time/height lock; must be `0` for immediate spend

**`CellOutput`** — a cell being created.
- `capacity` — size in shannons (1 CKB = 100,000,000 shannons)
- `lock_script` — defines who can spend this cell
- `type_script` — optional rules governing the cell's data

**`OutPoint`** — unique cell identifier: `tx_hash + index`.

**`CellDep`** — a read-only cell reference: `outpoint + dep_type` (0=code, 1=dep_group).

**`WitnessArgs`** — structured witness with three optional byte fields: `lock`, `input_type`, `output_type`.

---

### Transaction methods

**`hash() -> [u8;32]`**

Blake2b personalized hash (`"ckb-default-hash"`) of the Molecule-serialized `RawTransaction`. Witnesses are excluded — this is the stable identifier of the transaction body.

**`create_sighash() -> [u8;32]`**

Implements the CKB `sighash_all` algorithm. Commits to both the transaction body and all witnesses:

```
blake2b(
    raw_tx_hash          ← transaction body hash
    || 8-byte-LE-len || witnesses[0] with lock zeroed to 65 bytes
    || 8-byte-LE-len || witnesses[1]
    ...
)
```

`witnesses[0].lock` is replaced with 65 zero bytes before hashing. This is the placeholder slot where the real signature will be placed — zeroing it allows the digest to be computed before the signature exists.

**`create_signature(private_key: SecretKey) -> [u8;65]`**

Signs the sighash with a secp256k1 recoverable ECDSA signature (RFC 6979 deterministic nonce). Returns 65 bytes:

```
[ r (32 bytes) | s (32 bytes) | v (1 byte) ]
```

The recovery id `v` allows a verifier to reconstruct the public key from the signature alone, without it being transmitted separately.

**`sign(account: &Account, input_cells: &[CkbCell]) -> Result<Transaction>`**

Two-phase signing:
1. Ownership check — each input cell's `can_unlock_script(account)` must return true. Fails fast if any cell is not owned by the account.
2. Sign — calls `create_signature`, embeds the result into `witnesses[0].lock`, and returns the Molecule-serialized transaction.

**`validate_spend(input_index: usize, cells: &[CkbCell]) -> Result<()>`**

Full cryptographic validation of a single input spend:
1. `since == 0` — no time lock on the input
2. `input_index < witnesses.len()` — a witness slot exists
3. `witnesses[input_index].lock` is filled with exactly 65 bytes
4. The 65-byte signature is used to recover the signer's public key via secp256k1 ECDSA recovery
5. The recovered public key is hashed with Blake2b-160 to produce a pubkey hash
6. The pubkey hash must match `cells[input_index].lock_script.args` — i.e. the signature must have been made by the key that owns the cell

Returns `Ok(())` on success or a descriptive `Err` identifying the exact failure. This closes the security gap where any signature would pass a purely structural check — only the actual cell owner's signature is accepted.

**`transaction_builder() -> TransactionBuilder`**

Internal helper that assembles the full Molecule `TransactionBuilder` including witnesses, used by `sign()` to produce the final serialized transaction.

---

## consensus.rs

### MockLedger

An in-memory UTXO set: `HashMap<OutPoint, CellOutput>`.

| Method | Description |
|---|---|
| `birth_cell(outpoint, cell)` | Add a live cell. Errors if the outpoint is already live (prevents double-birth). |
| `kill_cell(outpoint)` | Remove a live cell. Errors if it doesn't exist (prevents double-spend). |
| `is_live(outpoint)` | Returns true if the outpoint exists in the live set. |
| `load(path)` | Deserialize ledger state from a JSON file. Returns an empty ledger if the file doesn't exist. |
| `save(path)` | Serialize ledger state to a JSON file. Creates parent directories if needed. |

The ledger serializes as a flat list of `{ outpoint, output }` entries since JSON requires string keys and `OutPoint` is a struct.

---

### Transaction lifecycle in the mock

```
1. CkbCell::can_unlock_script(account)        ← ownership check (data layer)
2. CKBTransaction::create_sighash()           ← compute digest
3. CKBTransaction::create_signature(sk)       ← sign digest
4. CKBTransaction::sign(account, cells)       ← embed signature → signed tx
5. CKBTransaction::validate_spend(i, cells)   ← cryptographic verification
6. MockLedger::kill_cell(input_op)            ← consume inputs
7. MockLedger::birth_cell(output_op, ..)      ← create outputs
```
