# CKBuilder (Tunji-Ogbara Timileyin)
## A log of what I learnt through implementation in code.

Fundamentals studied from: https://docs.nervos.org/docs/ckb-fundamentals

---

### Week 1: [27-04-2026 to 04-05-2026]

#### nervos_network

A ground-up Rust implementation of the CKB (Nervos Network) blockchain core, built to understand the Cell model, transaction lifecycle, and cryptographic primitives through direct implementation rather than abstraction.

**What it covers:**

- **Cell model** — cells as the fundamental unit of state: capacity, data, lock script, and type script. Lock scripts define ownership via a `code_hash` + `args` (pubkey hash) pair. `CkbCell` enforces unlock rules; `CkbScript` captures both lock and type script structure.

- **Accounts and key derivation** — 32-byte secret key → secp256k1 private key → compressed public key → Blake2b-160 pubkey hash. The pubkey hash is what goes into `lock_script.args` to assign cell ownership.

- **Address encoding** — full CKB bech32m address format derived from a lock script, matching the mainnet address standard.

- **Transactions** — the full `CKBTransaction` structure (version, cell_deps, header_deps, inputs, witnesses, outputs, output_data). Implements the CKB `sighash_all` algorithm and 65-byte recoverable ECDSA signatures.

- **Cryptographic validation** — `validate_spend` performs real signature verification: recovers the signer's public key from the signature, hashes it with Blake2b-160, and checks it against the cell's `lock_script.args`. Only the actual owner's signature passes.

- **Mock ledger** — a HashMap-based UTXO set backed by a JSON file. Supports `birth_cell` (create), `kill_cell` (spend), `is_live` (query), `load` and `save` for persistence across invocations.

- **Molecule serialization** — CKB binary wire format for transactions, generated from `.mol` schema files.

- **CLI tool (`ckb`)** — a clap-based interactive CLI for generating accounts, deriving addresses, managing the live cell ledger, and building/signing/validating transactions. Install with `cargo install --path nervos_network` and use from any directory.

See [`nervos_network/README.md`](nervos_network/README.md) for full CLI documentation and a worked Alice→Bob transfer example.

### Week 2: [04-05-2026 to 11-05-2026]
- Learning about the remaning terms on the block propagation like Block structure, proposal id's, the pow, nonce (solution of the pow puzzle)
- Learning and using the docs.nervos.org [repo](https://docs.nervos.org/docs/getting-started/quick-start#-build-a-dapp)
- Learning how to build smart contracts / scripts in CkB [link](https://docs.nervos.org/docs/getting-started/quick-start#-write-smart-contracts-scripts)

### Week 3: [11-05-2026 to 18-05-20226]
- Understanding / Building risc-v for ckb environment understanding.
- Truthixify intro to ckb 45 minutes notes.
- 