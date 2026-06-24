# data

Core CKB primitives — accounts, cells, and scripts — modelled as Rust structs independently of any specific transaction or network call.

Nothing in this folder talks to a node. These types are the building blocks used by both the CLI layer and the network layer.

---

## account.rs

Derives a CKB keypair from a 32-byte secret.

### Derivation chain

```
32-byte secret
    └── secp256k1 SecretKey
        └── compressed PublicKey (33 bytes)
            └── Blake2b-256  (personalized: "ckb-default-hash")
                └── first 20 bytes → pubkey_hash  (Blake160)
```

`Account::from_secret(secret: [u8; 32])` runs this entire chain and returns a struct holding both the private key and the pubkey_hash. The pubkey_hash is the value embedded into a secp256k1 lock script's `args` field to assert ownership of a cell.

### Why Blake160?

CKB uses a personalized Blake2b-256 hash (not the standard one) and takes only the first 20 bytes. This matches Ethereum's approach of truncating a public key hash to 20 bytes, keeping address sizes compact while retaining enough collision resistance for a UTXO system.

---

## cell.rs

Models the two fundamental CKB on-chain objects: scripts and cells.

### CkbScript

A script identifies a piece of executable code on-chain and the arguments to pass to it.

| Field | Type | Description |
|---|---|---|
| `code_hash` | `[u8; 32]` | Identifies which script binary to run |
| `hash_type` | `u8` | How `code_hash` is interpreted: `0`=Data, `1`=Type, `2`=Data1, `4`=Data2 |
| `args` | `[u8; 20]` | Input to the script — for a secp256k1 lock this is the owner's pubkey_hash |

**`pack() -> Script`** — converts to Molecule-serialized form for inclusion in a transaction (used by the network layer).

**`is_valid_hash_type() -> bool`** — guards against undefined hash_type values; only 0, 1, 2, and 4 are valid in the CKB spec.

### CkbCell

A cell is the fundamental unit of state on CKB — analogous to a UTXO in Bitcoin, but capable of carrying arbitrary data.

| Field | Description |
|---|---|
| `capacity` | Storage size in shannons (1 CKB = 100,000,000 shannons) |
| `data` | Arbitrary bytes stored in the cell |
| `lock_script` | Defines who can spend this cell (ownership) |
| `type_script` | Optional rules governing the cell's data (e.g. token contracts) |

**`can_unlock_script(account) -> bool`** — static ownership check. Returns true if the account's pubkey_hash matches `lock_script.args` and the hash_type is valid. This is the first gate before a spend can be attempted.

**`create_address(lock_script, network) -> Result<String>`** — encodes a lock script into a full CKB bech32m address. The payload is:
```
0x00 | code_hash (32 bytes) | hash_type (1 byte) | args (20 bytes)
```
Encoded with HRP `ckb` for mainnet or `ckt` for testnet/devnet.

**`lock_args() -> [u8; 20]`** — convenience accessor for `lock_script.args`, used when deriving change output ownership from an input cell.

---

## token.rs

Placeholder for future xUDT (extensible User Defined Token) support. Currently an empty struct with no logic.

xUDT is CKB's standard for fungible tokens. When implemented, this will model token amounts attached to cells via type scripts.

---

## mod.rs

Re-exports `Account`, `CkbCell`, and `CkbScript` at the `crate::data` level so other modules can import them without traversing the sub-module paths directly.
