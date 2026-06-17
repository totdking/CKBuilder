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