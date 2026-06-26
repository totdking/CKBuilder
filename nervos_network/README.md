# ckbuilder

A command-line toolkit for the [Nervos CKB](https://www.nervos.org/) blockchain.
Manage accounts, check balances, build and broadcast transactions, inspect cells and blocks, and query chain state — all from the terminal.

```sh
cargo install ckbuilder
```

---

## Features

- Works across **testnet**, **mainnet**, and **devnet** with a single tool
- Human-readable CKB amounts (e.g. `100` or `61.5`)
- bech32m address input — no manual pubkey hash extraction
- One-step `tx send` or an explicit **build → sign → broadcast** pipeline
- `tx decode` to inspect transaction JSON before signing
- `tx status` to check if a broadcast landed on-chain
- Automatic change output (protects against accidental miner donation)
- Dynamic fee scaling based on transaction size
- Multi-input cell aggregation (`--from` can be repeated)
- `--change-to` to redirect change to any address
- `--assert-owner` for early key mismatch detection
- Balance queries for any address (no private key required)
- Full cell inspection — capacity, lock script, type script, data
- Chain queries — tip block, block by number or hash
- Testnet faucet airdrop via `ckb airdrop`
- Raw JSON-RPC passthrough via `ckb rpc` for automation and scripting

---

## Install

**Requires:** Rust toolchain ([rustup.rs](https://rustup.rs))

### From crates.io

```sh
cargo install ckbuilder
```

### From source

```sh
git clone https://github.com/totdking/CKBuilder
cd CKBuilder/nervos_network
cargo install --path .
```

### Verify

```sh
ckb --help
```

---

## Networks

| Network | RPC | Address prefix |
|---------|-----|----------------|
| testnet (default) | https://testnet.ckb.dev | `ckt1q…` |
| mainnet | https://mainnet.ckb.dev | `ckb1q…` |
| devnet | http://localhost:8114 | `ckt1q…` |

```sh
ckb config set testnet
ckb config set mainnet
ckb config set devnet       # requires offckb running locally

ckb config get              # show active network + RPC URL
```

Use `--devnet` / `-d` on any command to target the local devnet without changing config:

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

### 2. Fund your testnet account

```sh
ckb airdrop              # request 10,000 CKB (default)
ckb airdrop 100000       # request 100,000 CKB
```

### 3. Check your balance

```sh
ckb balance                                  # from saved keypair
ckb balance ckt1qzda0cr08m85hc8jlnfp3...    # by address
ckb balance --utxos                          # show individual cells
```

### 4. Send CKB

```sh
ckb tx send --to ckt1qzda0cr08m85hc8jlnfp3... --amount 100
```

Selects cells automatically, adds change back to you, signs, and broadcasts in one step.

### 5. Inspect chain state

```sh
ckb tip                  # current block number and hash
ckb block 1              # block by number
ckb tx status <txhash>   # check if your transaction landed
```

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
ckb account show 0x<pubkey_hash>        # look up by raw pubkey_hash hex
```

### `airdrop` — request testnet CKB from the faucet

Testnet only. Sends a claim to `faucet.nervos.org` and prints the status.

```sh
ckb airdrop                  # request 10,000 CKB to your saved keypair address
ckb airdrop 100000           # request 100,000 CKB
ckb airdrop 300000           # request 300,000 CKB (faucet maximum)
ckb airdrop --address ckt1q... # send to a specific address
```

Valid amounts: `10000`, `100000`, `300000` (enforced by the faucet).

### `balance`

```sh
ckb balance                             # from saved keypair
ckb balance <address>                   # any bech32m address
ckb balance 0x<pubkey_hash>             # by raw pubkey_hash hex
ckb balance --utxos                     # list individual cells
ckb balance --devnet                    # query local devnet
```

### `cell` — inspect a live cell

```sh
ckb cell <txhash>:<index>          # show capacity, lock, and type script
ckb cell <txhash>:<index> --data   # also fetch cell data
```

Example output:

```
network:    testnet
outpoint:   0xabc...:0
capacity:   100.00000000 CKB  (10000000000 shannons)
lock:
  code_hash:  0x9bd7e06f...
  hash_type:  type
  args:       0xabcdef...
type:       (none)
```

### `tip` — show the current chain tip

```sh
ckb tip
ckb tip --rpc <url>    # override RPC endpoint
```

Example output:

```
network:    testnet
block:      21556356
hash:       0xe69cf65e...
timestamp:  1718400000000  (unix ms)
```

### `block` — fetch a block by number or hash

```sh
ckb block 1                     # by block number
ckb block 0x<32-byte-hash>      # by block hash
```

Example output:

```
network:    testnet
number:     1
hash:       0xd5ac7cf8...
timestamp:  1718400000000  (unix ms)
txs:        1
explorer:   https://pudge.explorer.nervos.org/block/0xd5ac7cf8...
```

### `rpc` — raw JSON-RPC passthrough

Call any [CKB JSON-RPC method](https://github.com/nervosnetwork/ckb/tree/develop/rpc) directly. Useful for automation, scripting, and LLM-driven tooling.

```sh
ckb rpc get_tip_block_number
ckb rpc get_block_by_number '["0x1"]'
ckb rpc get_transaction '["0xabc..."]'
ckb rpc get_live_cells '[{"script": {...}, "script_type": "lock"}, "asc", "0x10"]'
```

Returns the raw `result` field as pretty-printed JSON.

### `tx send` — build, sign, and broadcast in one step

```sh
ckb tx send \
  --to <address_or_pubkey_hash> \
  --amount <CKB>

# Options:
#   --keypair <path>    use a specific keypair file
#   --secret <hex>      use a raw secret key
#   --fee <shannons>    override fee (default: auto)
#   --rpc <url>         override RPC endpoint
#   --devnet            use local devnet
```

Minimum send amount is 61 CKB. Change is returned to you automatically.

### `tx build` — construct an unsigned transaction

```sh
ckb tx build \
  --from <txhash>:<index> \
  --to <address_or_pubkey_hash> \
  --amount <CKB> \
  --out tx.json

# Options:
#   --change-to <address>   redirect change to a different address
```

### `tx sign` — sign an unsigned transaction

```sh
ckb tx sign --tx tx.json                             # uses ~/.config/ckb/key.json
ckb tx sign --tx tx.json --keypair ~/my-key.json
ckb tx sign --tx tx.json --secret <32-byte-hex>
ckb tx sign --tx tx.json --out signed.json           # write to a new file

# Optional ownership check — fails early if the key doesn't match:
ckb tx sign --tx tx.json --assert-owner 0x<expected_pubkey_hash>
```

### `tx broadcast` — submit a signed transaction

```sh
ckb tx broadcast --tx signed.json
ckb tx broadcast --tx signed.json --devnet
```

Prints the tx hash and a link to the block explorer.

### `tx decode` — pretty-print a transaction file

Useful for reviewing a transaction before signing, or for debugging.

```sh
ckb tx decode --tx tx.json
```

Example output:

```
version:   0
inputs (1):
  [0]  0x7f921594...:0  since=0
outputs (2):
  [0]  100.00000000 CKB  lock: type/0x9bd7e06f../args=0xabcdef...  type: none
  [1]   50.12300000 CKB  lock: type/0x9bd7e06f../args=0x012345...  type: none
cell_deps (1):
  [0]  0xf8de3bb4...:0  dep_group
witnesses (1):
  [0]  0x5500000010...  (65 bytes)
tx hash:   0xdeadbeef...
```

### `tx hash` — print the tx hash of a transaction file

```sh
ckb tx hash --tx tx.json
```

### `tx status` — check on-chain status

```sh
ckb tx status <txhash>
ckb tx status <txhash> --rpc <url>
```

Example output:

```
status:       committed
block_hash:   0xabc...
block_number: 1234567
```

Possible statuses: `pending`, `proposed`, `committed`, `rejected`, `unknown`.

### `address` — derive a bech32m address from arbitrary lock script parameters

```sh
ckb address \
  --code-hash <32-byte-hex> \
  --hash-type <data|type> \
  --args <20-byte-hex>
```

---

## Full example — manual tx flow

The explicit alternative to `tx send`: build → sign → broadcast separately so you can inspect the transaction at each step.

```sh
# 1. Find an input cell
ckb balance --utxos

# 2. Build the unsigned transaction
ckb tx build \
  --from 7f9215948489c6567cd5e5c475f7f7787e3110dcb160137423fc5f55c384cc49:0 \
  --to ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq... \
  --amount 100 \
  --out send.json

# 3. Inspect it before signing
ckb tx decode --tx send.json

# 4. Sign it
ckb tx sign --tx send.json

# 5. Broadcast
ckb tx broadcast --tx send.json

# 6. Confirm it landed
ckb tx status <txhash>
```

---

## Local devnet (offckb)

```sh
npx offckb node          # start the devnet on localhost:8114
npx offckb accounts      # list pre-funded test accounts
```

Then use `--devnet` on any command, or set it as the active network:

```sh
ckb config set devnet
ckb balance
```

---

## File locations

| File | Purpose |
|------|---------|
| `~/.config/ckb/key.json` | Default keypair (secret key + addresses) |
| `~/.config/ckb/config.json` | Active network setting |

> Keep `key.json` secret. Never commit it to version control.

---

## License

MIT
