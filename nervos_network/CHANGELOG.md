# Changelog

All notable changes to `ckbuilder` are documented here.

## [0.1.3] - 2026-06-26

### Added
- `ckb airdrop [amount] [--address]` — request testnet CKB from faucet.nervos.org. Valid amounts: 10000, 100000, 300000 CKB
- `ckb cell <txhash>:<index> [--data]` — inspect any live cell: capacity, lock script, type script, and optional data
- `ckb tip` — show the current chain tip (block number + hash)
- `ckb block <number|hash>` — fetch a block by decimal number or 0x-prefixed hash
- `ckb rpc <method> [params]` — raw JSON-RPC passthrough for any CKB node method, returns pretty-printed JSON
- `ckb tx decode --tx <file>` — pretty-print a transaction file (inputs, outputs, scripts, witnesses, hash) before signing
- `ckb tx status <txhash>` — check on-chain status of a transaction: pending / proposed / committed / rejected

## [0.1.2] - 2026-06-01

### Fixed
- Resolved all clippy warnings blocking CI
- Ran `cargo fmt` across the workspace

## [0.1.1] - 2026-05-01

### Added
- Published to crates.io with installation instructions
- Added LICENSE and crates.io metadata

## [0.1.0] - 2026-04-01

### Added
- `ckb account new / show` — keypair generation and management
- `ckb balance` — query CKB balance for any address
- `ckb tx build / sign / broadcast / send / hash` — full transaction lifecycle
- `ckb address` — derive bech32m address from arbitrary lock script parameters
- `ckb config get / set` — switch between testnet, mainnet, and devnet
- `--devnet` flag on all commands for local offckb targeting
