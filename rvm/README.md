# RVM — RISC-V Virtual Machine

Building a RISC-V Instruction Set Simulator (ISS) is the standard rite of passage for truly understanding the architecture. Using Rust gives you memory-safety guarantees that C-based emulators simply can't offer, making it an ideal choice for modelling hardware state.

The goal of this project is to avoid building a generic emulator. Instead, build a focused **RV32I (Base Integer User-Level) interpreter** that executes a raw binary compiled from C or Assembly, while letting you observe every register and memory state change as it happens.

---

## Phase 1 — The Core Execution Loop

Start by modelling the hardware components as Rust structs:

| Component | Representation |
|-----------|---------------|
| Registers | `[u32; 32]` — note that `x0` is hardwired to zero |
| Program Counter | `u32` pointing to the current instruction |
| Memory | `Vec<u8>` or a fixed-size array simulating RAM |

The execution loop follows three steps on every tick:

1. **Fetch** — read 4 bytes from memory at the address held by the PC.
2. **Decode** — use bit masking and shifting to extract the opcode, `rd`, `rs1`, `rs2`, and any immediate values.
3. **Execute** — match on the opcode and perform the operation (e.g. `ADD`, `BEQ`, `LUI`).

---

## Phase 2 — Mastering Instruction Decoding

RISC-V uses a regular instruction encoding, but **immediate encoding is where most beginners get stuck**. The immediate bits are deliberately scrambled across the instruction word to minimise hardware fan-out.

**Your task:** implement sign-extension logic for all five instruction formats — I, S, B, U, and J-type.

> **Rust tip:** use a trait or an enum for instruction types to keep your `match` arms clean and maintainable.

---

## Phase 3 — System Calls and I/O

A simulator that only moves numbers between registers is effectively blind. You need a way to observe output.

- **ToHost mechanism** — define a special memory address (e.g. `0x80001000`). Any write to that address prints the character to the host's stdout.
- **ELF loading** — use the [`elf` crate](https://crates.io/crates/elf) to parse ELF files and load program sections directly into simulated RAM, instead of manually wrangling raw binaries.

---

## Common Pitfalls

These are the issues that trip up almost every first implementation:

### Sign Extension
When decoding a 12-bit immediate, you must manually propagate the sign bit (bit 11) into all upper bits of your 32-bit Rust value. Miss this and every jump or offset calculation will be catastrophically wrong.

### Memory Alignment
RISC-V permits unaligned memory access, but many simple implementations (and the spec itself) assume 4-byte alignment for instructions. Decide early whether you will enforce `PC % 4 == 0`.

### Wrap-around Arithmetic
Use `wrapping_add()` and `wrapping_sub()` everywhere. The standard `+` operator panics on overflow in Rust's debug mode — that is not how real hardware behaves.

---

## Levelling Up — Cycle-Accurate Pipeline

Once the interpreter feels comfortable, consider modelling a **cycle-accurate 5-stage pipeline**:

```
IF (Fetch) → ID (Decode) → EX (Execute) → MEM (Memory Access) → WB (Write Back)
```

The real challenge here is handling **hazards**:

- **Data hazards** — requires forwarding logic between pipeline stages.
- **Control hazards** — requires branch prediction or pipeline flushing.

This is where you stop reading about RISC-V and start truly understanding why it was designed the way it was.

---

## Assumptions

- Target architecture: **RV32I** (32-bit base integer set).
- You have a cross-compiler available (e.g. `riscv64-unknown-elf-gcc`) to generate test binaries.


