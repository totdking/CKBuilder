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

### Week 4 : [18-05-2026 to 25-05-2026]
- started building a RV64IMC similar to cvm environment to help with security in low level ckb
- Learnt about how instructions bit are formed and selected when passed into vm's.
- Started reading the risc-v manual
- learnt about opcodes, the use, how bits are isolated and masked using bit-shift and OR operations
- Learnt how bytes are read and collected from memory with how the program counter helps in getting instructions to be used for said memory
- Learnt about how cycles are to be used in gas for ckb and how to prevent cycle gas exhaustion through unbounded computation

### Week 5: [25-05-2026 to 1-06-2026]
- Learnt about ix types for risc-v and the ones that ckb uses and how it uses them in ix execution
- While executing a JAL (Jump and Link) ix, I learnt about how the immediate storage in the vm handles bit scrambling.
- Learnt about how to use arithmetic right shift(right shift on signed integers) to handle -ve / +ve jumps in memory.

### Week 6: [1-06-2026 to 7-06-2026]
- Learnt the difference between instruction format types (R/I/S/B/U/J) and instruction functional groups — format describes the bit layout, opcode identifies the functional group (e.g. OP-IMM, SYSTEM, LOAD), and multiple groups can share the same layout.
- Learnt that the opcode (bits 6:0) routes an instruction to its functional group (the "department"), while funct3 and funct7 identify the exact operation within that group — two-level decode: opcode → group, funct3/funct7 → specific instruction.
- Learnt that ECALL and EBREAK share I-type bit layout but use their own SYSTEM opcode (`0b1110011`), separate from OP-IMM (`0b0010011`) — "I-type" only describes the shape of the bits, not the purpose.
- Learnt that store instructions (S-type) have no `rd` field — the bits where `rd` would sit are repurposed for the upper half of the immediate, since stores write to memory via `rs1 + imm` using the value in `rs2`.
- Learnt that the sign bit for all immediates is always bit 31 (the MSB) across every instruction format — a deliberate RISC-V design choice to allow sign-extension to begin before the format is fully decoded.
- Learnt that while the sign bit is always bit 31, the reassembly of the full immediate differs per format — I-type is straightforward, S/B-types split the immediate across two fields, and J-type scrambles bits to maximise overlap with other formats.
- Built the instruction dispatcher in `instructions.rs` — a function that extracts the opcode and matches it to an `Instruction` enum variant, returning `Ok(variant)` for known opcodes and `Err` with the raw binary opcode for unknowns.

### Week 7: [8-06-2026 to 15-06-2026]
- Learnt that `memory: Vec<u8>` is a flat byte array and every address is just an index into it — address `0x108` means `memory[0x108]`, nothing more.
- Learnt that a load computes `addr = registers[rs1] + imm`, reads N bytes from `memory[addr]`, assembles them little-endian into a `u64`, and writes the result into `rd` — the same byte-assembly technique already used in `fetch_ix_at`.
- Learnt that a store computes the same address formula but moves data in the opposite direction: splits a register value into N bytes with `to_le_bytes()` and writes them into `memory[addr]`.
- Learnt that loads are I-type (use `get_imm_i`) while stores are S-type (use `get_imm_s`) — the immediate is split across two bit fields in S-type because the `rd` slot is repurposed for `rs2`.
- Learnt that stores have no `rd` — `rs2` is the data source and `rs1` is the base address, making the assembly syntax read as data-first, address-second (`SW x3, -4(x2)`).
- Learnt the distinction between signed and unsigned load variants: `LW` sign-extends 4 bytes to 64 bits (so `0xFFFFFFFF` becomes `-1`), `LWU` zero-extends the same bytes (becoming `+4294967295`), and `LD` reads all 8 bytes with no extension needed.
- Learnt that stores have no unsigned variants — narrowing a 64-bit register to 1/2/4 bytes always just truncates, so the signed/unsigned distinction is meaningless.
- Implemented `load()` and `store()` in `ckbvm.rs` — both dispatch on `funct3` to select byte width, use `wrapping_add` on a signed immediate for correct negative offsets, and advance `pc` after execution.

### Week 8: [15-06-2026 - 21-06-2026]

#### What I learnt

**The Cell Model is a pure UTXO system**

Every CKB account balance is a collection of individual cells. There is no single balance field — your "balance" is the sum of capacities across all live cells you own. When you "send" CKB, you consume one or more cells as inputs and create new cells as outputs. Cells that are not explicitly created as outputs cease to exist.

**The dep_group tx hash is network-specific**

Every network (mainnet, Pudge testnet, offckb devnet) has its own genesis block, which means the transaction that deploys the secp256k1 system script sits at a different tx hash on each network. Using the wrong hash causes a `TransactionFailedToResolve` error. The code hash of the secp256k1 binary itself is the same everywhere (it is derived from the binary, not the genesis), but the dep_group pointer must match the network.

**outpoint = txhash:index, and index resets per transaction**

A cell is uniquely identified by the tx hash that created it and its position in that transaction's output array. If a single transaction creates 5 outputs, they are indexed 0 through 4. A different transaction that also creates outputs starts back at index 0. Two cells can share an index as long as they were created by different transactions.

**The change output problem — or how I accidentally donated CKB to a miner**

This was the most important practical lesson. When building a transaction manually with `tx build`, I consumed a 200 CKB cell to send 100 CKB to myself. The transaction had only one output (the 100 CKB recipient cell). The remaining 100 CKB had nowhere to go — CKB's rules state that all input capacity that is not assigned to an output cell goes to the miner as a fee. There was no error, no warning — the transaction succeeded and 100 CKB silently became a miner reward.

The fix: `tx build` fetches the input cell's capacity, calculates the leftover (`input_capacity - amount - fee`), and creates a second output cell (the change output) sending that remainder back to the same address that owned the input cell. The sender's address is derived directly from the input cell's `lock_script.args` by querying the node — no private key is needed.

**CKB address = encoded lock script**

A CKB address is not an independent identifier — it is a bech32m encoding of a full lock script. The payload is `0x00 | code_hash (32 bytes) | hash_type (1 byte) | args (20 bytes)`. The `args` field is what uniquely identifies the owner (the pubkey_hash). This means decoding an address and extracting `payload[34..54]` gives you the pubkey_hash directly, with no additional derivation.

**offckb devnet has a deterministic genesis**

Every offckb devnet starts from the same genesis block, so the dep_group tx hash for devnet is always the same fixed value (`4d804f14...`). This is unlike a real chain where genesis is variable — offckb intentionally freezes the genesis so developers get a predictable local environment every time.

#### What I built

**`ckb` CLI**

A clap-based command-line tool that accepts human-readable inputs throughout:

- `balance` accepts any bech32m address or pubkey_hash — no private key needed for a read-only query
- `--to` accepts a full bech32m address (`ckt1q...`)
- `--amount` accepts CKB notation (`100` or `61.5`)
- `tx build` takes three flags: `--from`, `--to`, `--amount`
- `tx sign` accepts `--keypair` or `--secret` and falls back to `~/.config/ckb/key.json` by default
- `--assert-owner` flag on `tx sign` for early ownership mismatch detection

**Devnet as a first-class network**

`Network::Devnet` targets offckb on `localhost:8114`. A global `--devnet` / `-d` flag lets any command hit the local devnet without changing the saved config. The dep_group tx hash for the offckb genesis is hardcoded so devnet transactions resolve correctly.

**Automatic change output in `tx build`**

`tx build` fetches the input cell via `get_live_cell` RPC, reads its capacity and lock args, and calculates the change (`input_capacity - amount - fee`). A second output cell is created automatically, returning the change to the original cell owner. If the change would fall below 61 CKB (the minimum cell capacity), it is absorbed into the miner fee rather than producing an invalid dust cell.

**`tx broadcast` command**

`tx broadcast --tx signed.json` reads a signed transaction JSON file, submits it via the `send_transaction` RPC, and prints the tx hash and a block explorer link.

**Multi-input `tx build`**

`--from` accepts multiple values (`--from hash:0 --from hash:1 ...`). All specified input cells are fetched, their capacities summed, and change is calculated from the total. A witness is created for each input.

**Dynamic fee estimation**

`estimate_fee(n_inputs, n_outputs)` computes the fee from the approximate serialized byte size of the transaction multiplied by CKB's minimum fee rate of 1000 shannons/KB, clamped between 1000 and 100,000 shannons. Both `tx build` and `tx send` use this by default; `--fee` overrides it.

**CI/CD pipeline**

`.github/workflows/ci.yml` runs on every push and pull request: format check (`cargo fmt`), build, tests, and clippy with `-D warnings`.

### Week 9: [21-06-2026 to 26-06-2026]

Extended `ckbuilder` from a transaction-building tool into a fuller chain-inspection and dev-utility CLI.

**What I learnt**

- **Faucets are just another RPC-backed service** — `faucet.nervos.org` accepts a fixed set of testnet claim amounts (10000 / 100000 / 300000 CKB) per address. Wiring up `ckb airdrop` was less about CKB internals and more about treating the faucet as an external API with its own validation rules.
- **A raw RPC passthrough is a force multiplier for debugging** — rather than adding a dedicated command for every CKB node method, `ckb rpc <method> [params]` forwards arbitrary JSON-RPC calls straight to the node and pretty-prints the response. This covers any method not yet wrapped by a first-class command.
- **Transaction status is a distinct query from transaction existence** — `get_transaction` returns not just the transaction body but a status enum (`pending` / `proposed` / `committed` / `rejected`). `ckb tx status` surfaces just that enum, which is what you actually want to know after broadcasting.
- **Decoding before signing catches mistakes early** — `ckb tx decode` pretty-prints a transaction file's inputs, outputs, scripts, and witnesses so a builder can sanity-check what they're about to sign, rather than discovering an error only after broadcast fails.

**What I built**

- `ckb airdrop [amount] [--address]` — request testnet CKB from the faucet
- `ckb cell <txhash>:<index> [--data]` — inspect any live cell's capacity, lock script, type script, and optional data
- `ckb tip` — show the current chain tip (block number + hash)
- `ckb block <number|hash>` — fetch a block by decimal number or 0x-prefixed hash
- `ckb rpc <method> [params]` — raw JSON-RPC passthrough for any CKB node method
- `ckb tx decode --tx <file>` — pretty-print a transaction file before signing
- `ckb tx status <txhash>` — check on-chain status of a transaction

See [`nervos_network/CHANGELOG.md`](nervos_network/CHANGELOG.md) for the full versioned changelog and [`nervos_network/README.md`](nervos_network/README.md) for command documentation.
