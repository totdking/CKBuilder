# CKB-VM RV64IMC Construction Plan

## Phase 0: Environment Setup and Bitwise Kata
**Objective:** Establish the toolchain and master the binary arithmetic required to parse CPU instructions.
- [ ] **Install Toolchains:** Install Rust and the `ckb-riscv-gnu-toolchain` (specifically `riscv64-unknown-elf-gcc` to compile your test contracts).
- [ ] **Bitwise Isolation:** Write standalone Rust functions to practice masking (`&`) and shifting (`<<`, `>>`).
  - *Test:* Write a function that accepts the 32-bit integer `0x00A58533` (an `add` instruction) and isolates bits 7 through 11 to extract the destination register.
- [ ] **Documentation:** Download the *RISC-V Instruction Set Manual (Volume I)*. Bookmark the opcode maps.

## Phase 1: The Bare-Metal Loop (The MVP)
**Objective:** Construct a finite state machine that mutates an array based on hardcoded byte instructions.
- [ ] **State Modelling:** Define the core `CkbVm` struct.
  - `registers: [u64; 32]` (Note: enforce `registers[0]` as hardwired to 0).
  - `pc: u64` (Program Counter).
  - `memory: Vec<u8>`.
- [ ] **Fetch & Execute Loop:** Write the `while` loop that reads 4 bytes from `memory` at `pc`.
- [ ] **Implement 3 Opcodes:**
  - `ADDI` (Add Immediate): Decode opcode, extract the immediate, add to `rs1`, store in `rd`.
  - `JAL` (Jump and Link): Decode offset, mutate `pc`.
  - `ECALL` (Environment Call): Print register state and `break` the loop.
- [ ] **Verification:** Hardcode a byte array representing `ADDI`, `ADDI`, `ECALL`. Run the loop and verify the array mutation.

## Phase 2: Complete the RV64I Baseline
**Objective:** Map the standard 64-bit integer instruction set required for basic C logic.
- [ ] **Instruction Decoding:** Create an `Instruction` enum representing the RISC-V formats (R-type, I-type, S-type, B-type, U-type, J-type). Write a dispatcher mapping 32-bit values to these enums.
- [ ] **Sign-Extension Logic:** Implement proper sign-extension. (e.g., propagating the 12th bit of an I-type immediate across the remaining 52 bits of a `u64`). *Failure here corrupts all branch calculations.*
- [ ] **Memory Operations:** Implement load/store instructions (`LD`, `SD`, `LW`, `SW`).
- [ ] **Control Flow:** Implement all branching instructions (`BEQ`, `BNE`, `BLT`, `BGE`).

## Phase 3: The CKB Bridge (Memory and Syscalls)
**Objective:** Restrict the generic RISC-V machine to emulate the exact constraints of the Nervos blockchain environment.
- [ ] **W^X Memory Paging:** Refactor `memory` from a flat `Vec<u8>` into simulated 4KB pages. Implement Write XOR Execute (W^X) protection.
  - *Trap logic:* Abort execution if the `pc` points to a non-executable page, or if a store instruction targets a non-writable page.
- [ ] **Mock the Blockchain State:** Create a mock `Transaction` struct containing dummy Cell data.
- [ ] **CKB Syscalls (`ECALL` expansion):**
  - Implement Syscall 2092 (`ckb_load_cell_data`): When `a7 == 2092`, pause execution, fetch data from your mock `Transaction` struct, write it into the VM's memory buffer, and resume.
  - Implement Syscall 93 (`ckb_exit`): Terminate execution and return the status code.

## Phase 4: The "C" Extension (Compressed Instructions)
**Objective:** Support the 16-bit instruction format. CKB heavily relies on the RV64C extension to minimize on-chain binary size and reduce storage costs.
- [ ] **Dynamic Fetching:** Modify the Fetch step. Check the lowest 2 bits of the fetched instruction.
  - If bits are `11`: Fetch 4 bytes (32-bit instruction).
  - If bits are `00`, `01`, or `10`: Fetch 2 bytes (16-bit instruction).
- [ ] **Decompression Translation:** Write a decompression module. Do not write a separate execution engine for C-type instructions; instead, translate the 16-bit compressed bytes into their equivalent 32-bit RV64I counterparts, then pass them to your existing execution loop.

## Phase 5: ELF Loading and Cycle Counting
**Objective:** Execute an actual compiled CKB smart contract and measure its deterministic computational cost.
- [ ] **ELF Parsing:** Use the `elf` Rust crate to parse a `.elf` smart contract compiled via the GNU toolchain.
- [ ] **Memory Mapping:** Load the `.text`, `.data`, and `.rodata` sections of the ELF file into their respective W^X memory pages in your VM.
- [ ] **Cycle Metering (Gas Equivalent):** Add a `cycles: u64` counter.
  - Assign costs based on the CKB Cycle Limits model (e.g., arithmetic = 1 cycle, branching = 3 cycles, memory loads = heavier penalty).
  - Add a `MAX_CYCLES` threshold that aborts execution (simulating an `Out of Gas` error).