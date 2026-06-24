# cli

The CLI layer — parses user commands, validates inputs, and dispatches to the RPC layer to interact with a live CKB node.

Built with [`clap`](https://docs.rs/clap) derive macros. The binary is named `ckb`.

---

## mod.rs

The main command module. Contains all argument structs, helper functions, and the dispatch logic that connects user input to network calls.

### Commands

| Command | Description |
|---|---|
| `ckb account new` | Generate or import a keypair, save to `~/.config/ckb/key.json` |
| `ckb account show` | Show your saved keypair, or look up any address / pubkey_hash |
| `ckb balance` | Check the CKB balance of any address on the active network |
| `ckb address` | Advanced: derive a bech32m address from arbitrary lock script parameters |
| `ckb config get/set` | View or change the active network (testnet / mainnet / devnet) |
| `ckb tx send` | Build, sign, and broadcast a transfer in one step |
| `ckb tx build` | Construct an unsigned transaction and write it to a JSON file |
| `ckb tx sign` | Sign an unsigned transaction JSON file with a private key |
| `ckb tx broadcast` | Submit a signed transaction JSON file to the network |
| `ckb tx hash` | Print the tx hash of any transaction JSON file |

### Global flag

`--devnet` / `-d` overrides the active network to `http://localhost:8114` (offckb) for a single command without changing the saved config.

### Helper functions

| Function | Purpose |
|---|---|
| `parse_ckb_amount(s)` | Accepts `"100"` or `"100.5"` → shannons; enforces 61 CKB minimum |
| `parse_addr_or_pubkey_hash(s)` | Accepts a bech32m address or 20-byte hex → `[u8; 20]` pubkey_hash |
| `parse_outpoint_str(s)` | Parses `"txhash:index"` into `([u8;32], u32)` |
| `resolve_secret(keypair, secret)` | Loads a private key from `--secret` hex, `--keypair` path, or the default `~/.config/ckb/key.json` |
| `active_rpc(cfg, override, devnet)` | Returns the RPC URL to use, respecting `--devnet` and `--rpc` overrides |
| `active_network(cfg, devnet)` | Returns the `Network` enum value to use for the current command |
| `estimate_fee(n_inputs, n_outputs)` | Dynamically calculates transaction fee based on size |

### Stored files

| File | Contents |
|---|---|
| `~/.config/ckb/key.json` | Keypair: secret key, pubkey_hash, testnet address, mainnet address |
| `~/.config/ckb/config.json` | Active network setting (`testnet` / `mainnet` / `devnet`) |

---

## hex_serde.rs

Custom [`serde`](https://docs.rs/serde) serializers and deserializers for byte arrays, which serde cannot handle automatically.

CKB data types like tx hashes, code_hashes, and lock args are fixed-size byte arrays (`[u8; 32]`, `[u8; 20]`). JSON has no native binary type, so these are encoded as `0x`-prefixed hex strings on the wire.

### Modules

| Module | Type | Serialized form |
|---|---|---|
| `hex_serde::array32` | `[u8; 32]` | `"0x<64 hex chars>"` |
| `hex_serde::array20` | `[u8; 20]` | `"0x<40 hex chars>"` |
| `hex_serde::vec_bytes` | `Vec<u8>` | `"0x<hex>"` |
| `hex_serde::opt_vec_bytes` | `Option<Vec<u8>>` | `"0x<hex>"` or `null` |

These are applied via `#[serde(with = "crate::cli::hex_serde::array32")]` annotations on struct fields in `CkbScript`, `OutPoint`, `CKBTransaction`, and `WitnessArgs`. They ensure the JSON saved by `tx build` and `tx sign` can be round-tripped correctly and is human-readable.
