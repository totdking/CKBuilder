# ckbuilder

A command-line toolkit for the [Nervos CKB](https://www.nervos.org/) blockchain.
Manage accounts, check balances, and build, sign, and broadcast transactions on testnet, mainnet, or a local devnet — all from the terminal.

```sh
cargo install ckbuilder
```

---

## Features

- Works across **testnet**, **mainnet**, and **devnet** with a single tool
- Human-readable CKB amounts (e.g. `100` or `61.5`)
- bech32m address input — no manual pubkey hash extraction
- One-step `tx send` or an explicit **build → sign → broadcast** pipeline
- Inspect transaction JSON before signing
- Automatic change output (protects against accidental miner donation)
- Dynamic fee scaling based on transaction size
- Multi-input cell aggregation (`--from` can be repeated)
- `--change-to` to redirect change to any address
- `--assert-owner` for early key mismatch detection
- Balance queries for any address (no private key required)

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

### 2. Check your balance

```sh
ckb balance ckt1qzda0cr08m85hc8jlnfp3...   # by address
ckb balance                                  # from saved keypair
ckb balance --utxos                          # show individual cells
```

### 3. Send CKB

```sh
ckb tx send --to ckt1qzda0cr08m85hc8jlnfp3... --amount 100
```

Selects cells automatically, adds change back to you, signs, and broadcasts in one step.

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

### `balance`

```sh
ckb balance                             # from saved keypair
ckb balance <address>                   # any bech32m address
ckb balance 0x<pubkey_hash>             # by raw pubkey_hash hex
ckb balance --utxos                     # list individual cells
ckb balance --devnet                    # query local devnet
```

### `tx send` — build, sign, and broadcast in one step

```sh
ckb tx send \
  --to <address_or_pubkey_hash> \
  --amount <CKB>

# Options:
#   --keypair <path>    use a specific keypair file
#   --secret <hex>      use a raw secret key
#   --fee <shannons>    override fee (default: 1000)
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

Fetches the input cell from the node, calculates change, and writes the unsigned tx to JSON.

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

### `tx hash` — print the tx hash of a transaction file

```sh
ckb tx hash --tx tx.json
```

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

# 3. Sign it
ckb tx sign --tx send.json

# 4. Broadcast
ckb tx broadcast --tx send.json
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