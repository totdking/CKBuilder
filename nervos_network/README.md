# ckb — CKB Developer Toolkit

A command-line tool for working with the [Nervos CKB](https://www.nervos.org/) blockchain. Manage accounts, check balances, and build/sign/broadcast transactions on testnet, mainnet, or a local devnet — all from the terminal.

---

## Install

**Requirements:** Rust toolchain ([rustup.rs](https://rustup.rs))

```sh
git clone <repo-url>
cd nervos_network
cargo install --path .
```

This builds a release binary and puts `ckb` on your PATH. To update after code changes, re-run the same command.

```sh
ckb --help
```

---

## Networks

The CLI supports three networks. Switch with `ckb config set`:

| Network | RPC | Address prefix |
|---------|-----|----------------|
| testnet (default) | https://testnet.ckb.dev | `ckt1q...` |
| mainnet | https://mainnet.ckb.dev | `ckb1q...` |
| devnet | http://localhost:8114 | `ckt1q...` |

```sh
ckb config set testnet
ckb config set mainnet
ckb config set devnet       # requires offckb running locally

ckb config get              # show active network + RPC URL
```

Use `--devnet` / `-d` on any command to hit the local devnet without changing the config:

```sh
ckb balance <address> --devnet
```

---

## Quickstart

### 1. Generate an account

```sh
ckb account new
```

Saves your keypair to `~/.config/ckb/key.json`. View it later:

```sh
ckb account show
```

### 2. Check your balance

```sh
# by address
ckb balance ckt1qzda0cr08m85hc8jlnfp3...

# using your saved keypair
ckb balance

# show individual cells (UTXOs)
ckb balance --utxos
```

### 3. Send CKB (one step)

```sh
ckb tx send --to ckt1qzda0cr08m85hc8jlnfp3... --amount 100
```

Uses your saved keypair at `~/.config/ckb/key.json`. Automatically selects cells, adds change back to you, signs, and broadcasts.

---

## Commands

### `account`

```sh
ckb account new                         # generate a new keypair
ckb account new --secret <32-byte-hex>  # import an existing secret key
ckb account new --out ~/my-key.json     # save to a custom path
ckb account new --force                 # overwrite existing keypair

ckb account show                        # show your saved keypair
ckb account show <address>              # look up any address (no private key needed)
ckb account show 0x<pubkey_hash>        # same, by raw pubkey_hash hex
```

### `balance`

```sh
ckb balance                             # your saved keypair
ckb balance <address>                   # any bech32m address
ckb balance 0x<pubkey_hash>             # by raw pubkey_hash hex
ckb balance --utxos                     # list individual cells
ckb balance --devnet                    # query local devnet
```

### `tx send` — build, sign, broadcast in one step

```sh
ckb tx send \
  --to <address_or_pubkey_hash> \
  --amount <CKB>                        # e.g. 100 or 61.5

# Options:
#   --keypair <path>    use a specific keypair file
#   --secret <hex>      use a raw secret key
#   --fee <shannons>    override fee (default: 1000)
#   --rpc <url>         override RPC endpoint
#   --devnet            use local devnet
```

Change (leftover capacity) is automatically returned to you. Minimum send amount is 61 CKB.

### `tx build` — construct an unsigned transaction

Use this when you want to review or modify the transaction before signing.

```sh
ckb tx build \
  --from <txhash>:<index> \
  --to <address_or_pubkey_hash> \
  --amount <CKB> \
  --out tx.json                         # default: tx.json
```

Options:
- `--change-to <address>` — send change to a different address (default: back to the `--from` cell owner)

Fetches the input cell from the node, calculates change automatically, and writes the unsigned tx to a JSON file.

### `tx sign` — sign an unsigned transaction file

```sh
ckb tx sign --tx tx.json               # uses ~/.config/ckb/key.json
ckb tx sign --tx tx.json --keypair ~/my-key.json
ckb tx sign --tx tx.json --secret <32-byte-hex>
ckb tx sign --tx tx.json --out signed.json   # write to a new file instead of overwriting
```

Optional ownership check — fails early with a clear message if the key doesn't match:
```sh
ckb tx sign --tx tx.json --assert-owner 0x<expected_pubkey_hash>
```

### `tx broadcast` — submit a signed transaction

```sh
ckb tx broadcast --tx signed.json
ckb tx broadcast --tx signed.json --devnet
```

Prints the tx hash and a link to the block explorer.

### `tx hash` — print the tx hash of any transaction file

```sh
ckb tx hash --tx tx.json
```

### `address` — advanced: derive a bech32m address from arbitrary lock script parameters

```sh
ckb address \
  --code-hash <32-byte-hex> \
  --hash-type <data|type> \
  --args <20-byte-hex>
```

---

## Full example — manual tx flow

This is the exploded version of `tx send`: build → sign → broadcast separately, so you can inspect the transaction at each step.

```sh
# 1. Check your UTXOs to pick an input cell
ckb balance --utxos

# 2. Build the unsigned transaction
ckb tx build \
  --from 7f9215948489c6567cd5e5c475f7f7787e3110dcb160137423fc5f55c384cc49:0 \
  --to ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq... \
  --amount 100 \
  --out send.json

# Output shows:
#   tx hash, input amount, send amount, change amount, fee

# 3. Sign it
ckb tx sign --tx send.json

# 4. Broadcast
ckb tx broadcast --tx send.json

# Explorer link is printed automatically
```

To send to someone else while keeping change for yourself:
```sh
ckb tx build \
  --from <your_cell>:<index> \
  --to <recipient_address> \
  --amount 100
# Change goes back to you automatically (derived from the --from cell's lock)
```

To redirect change to a different address:
```sh
ckb tx build \
  --from <cell>:<index> \
  --to <recipient> \
  --amount 100 \
  --change-to <change_address>
```

---

## Devnet (offckb)

Run a local CKB node with [offckb](https://github.com/ckb-ecofund/offckb):

```sh
npx offckb node          # starts the devnet on localhost:8114
npx offckb accounts      # pre-funded test accounts
```

Then use `--devnet` on any command or set it as your active network:

```sh
ckb config set devnet
ckb balance              # queries localhost:8114
```

---

## File locations

| File | Purpose |
|------|---------|
| `~/.config/ckb/key.json` | Default keypair (secret key + addresses) |
| `~/.config/ckb/config.json` | Active network setting |

> Keep `key.json` secret. Do not commit it to git.

---

## Week 8 — What I learnt and what I built

### What this toolkit adds that offckb and Orbital do not

**offckb** (`ckb-ecofund/offckb`) is a local devnet launcher. Its job is to spin up a CKB node on your machine and fund test accounts. The `offckb transfer` command exists but is tightly coupled to the local devnet — it is a convenience wrapper, not a general-purpose transaction tool. It does not work on testnet or mainnet, does not expose a manual build → sign → broadcast pipeline, does not protect against the missing change output problem, and cannot be installed as a standalone binary.

**Orbital** (`radiiplus/Orbital`) is a higher-level abstraction layer over CKB. It simplifies interactions through a library or framework API rather than through a developer terminal workflow.

This CLI fills a different gap — a network-portable, terminal-native toolkit for developers who want full control over every step of a CKB transaction without leaving the command line. The specific additions:

| Capability | offckb | Orbital | ckb (this) |
|---|:---:|:---:|:---:|
| Works on testnet, mainnet, devnet with one tool | — | — | ✓ |
| Human-readable CKB amounts (`100` not `10000000000`) | — | — | ✓ |
| bech32m address input (no manual pubkey_hash extraction) | — | — | ✓ |
| Manual build → sign → broadcast pipeline (file-based) | — | — | ✓ |
| Inspect transaction JSON before signing | — | — | ✓ |
| Automatic change output (protects against miner donation) | — | — | ✓ |
| Dynamic fee scaling with tx size | — | — | ✓ |
| Multi-input cell aggregation (`--from` repeated) | — | — | ✓ |
| `--change-to` redirect change to any address | — | — | ✓ |
| `--assert-owner` early key mismatch detection | — | — | ✓ |
| Balance query for any address (no private key required) | — | — | ✓ |
| Installable as a system binary (`cargo install ckbuilder`) | — | — | ✓ |
| CI/CD pipeline | — | — | ✓ |
| Private key never leaves the machine | ✓ | — | ✓ |
| Local devnet environment | ✓ | — | ✓ |

The most important distinction: because the build and sign steps are separated and file-based, a developer can build a transaction offline, inspect the JSON to verify every field (inputs, outputs, amounts, change), and only then sign and broadcast. offckb's transfer command and Orbital's abstractions collapse this into a single call where the internals are hidden. This CLI exposes the internals by design — the goal is understanding what CKB transactions actually contain, not abstracting it away.

---

### What I learnt

**The Cell Model is a pure UTXO system**

Every CKB account balance is a collection of individual cells. There is no single balance field — your "balance" is the sum of capacities across all live cells you own. When you "send" CKB, you consume one or more cells as inputs and create new cells as outputs. Cells that are not explicitly created as outputs cease to exist.

**The dep_group tx hash is network-specific**

Every network (mainnet, Pudge testnet, offckb devnet) has its own genesis block, which means the transaction that deploys the secp256k1 system script sits at a different tx hash on each network. Using the wrong hash causes a `TransactionFailedToResolve` error. The code hash of the secp256k1 binary itself is the same everywhere (it is derived from the binary, not the genesis), but the dep_group pointer must match the network.

**outpoint = txhash:index, and index resets per transaction**

A cell is uniquely identified by the tx hash that created it and its position in that transaction's output array. If a single transaction creates 5 outputs, they are indexed 0 through 4. A different transaction that also creates outputs starts back at index 0. Two cells can share an index as long as they were created by different transactions.

**The change output problem — or how I accidentally donated CKB to a miner**

This was the most important practical lesson. When building a transaction manually with `tx build`, I consumed a 200 CKB cell to send 100 CKB to myself. The transaction had only one output (the 100 CKB recipient cell). The remaining 100 CKB had nowhere to go — CKB's rules state that all input capacity not assigned to an output cell goes to the miner as a fee. There was no error, no warning — the transaction succeeded and 100 CKB silently became a miner reward.

The fix: after fetching the input cell's capacity, `tx build` now calculates the leftover (`input_capacity - amount - fee`), creates a second output cell (the change output) sending that remainder back to the same address that owned the input cell. The sender's address is derived directly from the input cell's `lock_script.args` by querying the node — no private key is needed.

**CKB address = encoded lock script**

A CKB address is not an independent identifier — it is a bech32m encoding of a full lock script. The payload is `0x00 | code_hash (32 bytes) | hash_type (1 byte) | args (20 bytes)`. The `args` field is what uniquely identifies the owner (the pubkey_hash). Decoding an address and extracting `payload[34..54]` gives the pubkey_hash directly, with no additional derivation.

**offckb devnet has a deterministic genesis**

Every offckb devnet starts from the same genesis block, so the dep_group tx hash for devnet is always the same fixed value (`4d804f14...`). This is unlike a real chain where genesis is variable — offckb intentionally freezes the genesis so developers get a predictable local environment every time.

---

### What I built

**Developer-friendly CLI overhaul**

Rewrote the entire CLI from a prototype requiring raw hex inputs into a tool that accepts human-readable values:

- `balance` now accepts any bech32m address or pubkey_hash — no private key needed for a read-only query
- `--to` accepts a full bech32m address (`ckt1q...`) rather than a raw 20-byte hex pubkey_hash
- `--amount` accepts CKB notation (`100` or `61.5`) rather than raw shannons (`10000000000`)
- `tx build` simplified from 6 flags down to 3: `--from`, `--to`, `--amount`
- `tx sign` accepts `--keypair` or `--secret` and falls back to `~/.config/ckb/key.json` by default
- `--assert-owner` flag on `tx sign` for early ownership mismatch detection

**Devnet as a first-class network**

Added `Network::Devnet` pointing to offckb on `localhost:8114`. Added a global `--devnet` / `-d` flag so any command can target the local devnet without changing the saved config. Hardcoded the correct dep_group tx hash for the offckb genesis.

**Automatic change output in `tx build`**

`tx build` fetches the input cell from the node via `get_live_cell` RPC, reads its capacity and lock args, calculates the change (`input_capacity - amount - fee`), and automatically creates a second output returning the change to the original cell owner. If the change would be below 61 CKB (the minimum cell capacity), it is absorbed into the miner fee rather than creating an invalid dust cell.

**`tx broadcast` command**

Added the missing link in the manual flow: `tx broadcast --tx signed.json` reads a signed transaction JSON file, submits it via `send_transaction` RPC, and prints the tx hash and block explorer link.

**Multi-input `tx build`**

`--from` now accepts multiple values (`--from hash:0 --from hash:1 ...`). All input cells are fetched, their capacities summed, and change is calculated from the total. Witnesses are created for each input.

**Dynamic fee estimation**

Replaced the hardcoded 1000-shannon constant with `estimate_fee(n_inputs, n_outputs)` — a formula based on the approximate serialized byte size of the transaction multiplied by CKB's minimum fee rate of 1000 shannons/KB, clamped between 1000 and 100,000 shannons. Both `tx build` and `tx send` now use dynamic fee, and `tx send --fee 0` (the new default) auto-estimates.

**CI/CD pipeline**

Added `.github/workflows/ci.yml` — runs on every push and pull request: format check (`cargo fmt`), build, tests, and clippy with `-D warnings`.

