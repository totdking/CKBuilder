# data

The data layer models the core CKB primitives that exist independently of any specific transaction — accounts, scripts, and cells.

---

## account.rs

Represents a CKB keypair derived from raw entropy.

**Derivation chain:**
```
32-byte secret
    └── secp256k1 SecretKey
        └── compressed PublicKey (33 bytes)
            └── Blake2b-256 (personalized: "ckb-default-hash")
                └── truncate to 20 bytes → pubkey_hash (Blake160)
```

`Account::from_secret(secret: [u8;32])` performs this entire derivation and returns a struct holding the private key and pubkey_hash. The pubkey_hash is what gets embedded into a lock script's `args` field to assert ownership of a cell.

`Address::from_script(lock_script: CkbScript)` is a thin wrapper that derives the bech32m address string from a lock script, delegating to `CkbCell::create_address`.

---

## cell.rs

Models the two fundamental CKB on-chain objects: cells and scripts.

### CkbScript

A script has three fields:

| Field | Type | Description |
|---|---|---|
| `code_hash` | `[u8;32]` | Blake2b hash identifying the script binary |
| `hash_type` | `u8` | How `code_hash` is interpreted: `0`=Data, `1`=Type, `2`=Data1, `4`=Data2 |
| `args` | `[u8;20]` | Script input — for a lock script this is the owner's pubkey_hash |

`CkbScript::pack()` converts the script to its Molecule-serialized form for inclusion in a transaction.

`CkbScript::is_valid_hash_type()` guards against undefined hash_type values (only 0, 1, 2, 4 are valid).

### CkbCell

A cell has capacity (shannons), optional data, a lock script (who owns it), and an optional type script (what rules govern it).

Key methods:

**`can_unlock_script(account: &Account) -> bool`**
Static ownership check — returns true if the account's pubkey_hash matches the lock script's args and the hash_type is valid. This is the first gate before any spend can occur.

**`create_address(lock_script: CkbScript) -> Result<String>`**
Encodes a lock script into a full CKB bech32m address. The payload is:
```
0x00 | code_hash (32 bytes) | hash_type (1 byte) | args (20 bytes)
```
encoded with HRP `ckb` (mainnet) or `ckt` (testnet).

**`consume_cell` / `create_cell`**
Wrappers over `MockLedger::kill_cell` and `MockLedger::birth_cell` — used to transition cell state during transaction execution.

**`is_live(ledger, outpoint) -> bool`**
Delegates to `MockLedger::is_live` to check whether a cell at a given outpoint exists in the live set.

---

## token.rs

Placeholder for future UDT (User Defined Token) logic. Currently an empty struct.
