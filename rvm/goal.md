# # RVM-CKB — Deep-Dive RISC-V Virtual Machine for Nervos Security & Optimization

## 1. Project Objective & Goal

The core objective of this project is to build a hyper-focused **RV64IMC (64-bit Base Integer with Multiplication and Compressed extensions) Interpreter** from scratch using Rust. 

Unlike generic hardware emulators, this simulator is explicitly engineered to replicate the **Nervos CKB-VM execution layer**. The ultimate goal is to create an isolated, deterministic sandbox that allows a software engineer or security researcher to observe every register mutation, trace memory paging, step through cryptographic system calls (`ecall`), and analyze cycle consumption (gas golfing) of compiled smart contract binaries (`.elf`).

### Strategic Career Goals for the CKB Ecosystem:
*   **Cycle Optimization Mastery:** Attain an intuitive understanding of compiler output to optimize zero-knowledge (ZK) verifiers and signature schemes, slashing execution costs on-chain.
*   **Low-Level Security Auditing:** Develop the capability to audit smart contracts at the bytecode level, hunting for memory alignment vulnerabilities, buffer overflows, and execution-layer exploits.
*   **Crypto-Agnostic Infrastructure Engineering:** Master the exact system-call boundary required to port advanced cryptography (Passkeys, Quantum-resistant signatures, SNARKs) onto the Nervos Network.

---

## 2. System Architecture & Component Mapping

The virtual hardware is modeled using clean, type-safe Rust structures to enforce strict state control:

| Component | Hardware Representation | CKB-VM Specific Rule / Constraint |
| :--- | :--- | :--- |
| **Registers** | `[u64; 32]` | Register `x0` is hardwired to `0`. Writes to it are silently discarded. |
| **Program Counter** | `u64` | Must support 2-byte alignment due to the Compressed (`C`) extension. |
| **Memory Architecture** | Page-mapped `Vec<u8>` | Enforces strict 4KB paging with Write XOR Execute (**W^X**) security permissions. |
| **Cycle Meter** | `u64` | Tracks execution costs; must trigger an abort if `MAX_CYCLES` is breached. |

---

## 3. Implementation Roadmap

### Phase 0: Environment Setup & Bitwise Foundations
- [ ] Install the Rust toolchain and the `ckb-riscv-gnu-toolchain` (specifically `riscv64-unknown-elf-gcc`).
- [ ] Master manual bit-masking (`&`, `|`) and bit-shifting (`<<`, `>>`) in Rust.
- [ ] *Milestone:* Write a function to parse `0x00A58533` (`add` instruction) and successfully extract `rd`, `rs1`, and `rs2` register indices using raw bitwise operations.

### Phase 1: The Core Execution Loop (The MVP)
- [ ] Model the core `CkbVm` state struct with registers, PC, and memory.
- [ ] Implement the atomic execution engine loop: **Fetch** $\rightarrow$ **Decode** $\rightarrow$ **Execute** $\rightarrow$ **Advance PC**.
- [ ] Implement the first three breakthrough instructions:
    *   `ADDI` (Add Immediate): To load values into registers.
    *   `JAL` (Jump and Link): To manipulate the Program Counter for control flow.
    *   `ECALL` (Environment Call): Act as a hard exit condition to break the loop and dump register states.

### Phase 2: Mastering Instruction Decoding & Sign Extension
- [ ] Design an `Instruction` enum parsing the core RISC-V formats: R, I, S, B, U, and J-type.
- [ ] Implement robust **sign-extension logic**. When decoding short bits (e.g., a 12-bit immediate), you must manually propagate the sign bit across all upper bits of your 64-bit Rust value. 
- [ ] Implement mandatory arithmetic operations (`ADD`, `SUB`, `MUL`), memory access (`LD`, `SD`, `LW`, `SW`), and conditional branching (`BEQ`, `BNE`, `BLT`, `BGE`).
- [ ] *Rust Tip:* Use wrapping arithmetic (`wrapping_add`, `wrapping_sub`) exclusively. Real hardware overflows naturally; standard Rust operators panic in debug mode.

### Phase 3: The CKB Bridge (Syscalls & W^X Paging)
- [ ] Refactor memory from a flat array to a 4KB paged model. Implement **W^X (Write XOR Execute)** rules: a memory page can be writable or executable, but never both simultaneously. 
- [ ] Create a mock `Transaction` struct containing dummy Input and Output Cells to act as external state.
- [ ] Expand the `ECALL` handler to recognize CKB System Calls:
    *   `Syscall 2092 (ckb_load_cell_data)`: Pause VM execution, read data from the mock transaction layer, write it to the VM memory buffer, and resume.
    *   `Syscall 93 (ckb_exit)`: Gracefully terminate the script execution pipeline with a status code.

### Phase 4: Supporting the "C" (Compressed) Extension
- [ ] Refactor the **Fetch** stage to evaluate the instruction length dynamically. Look at the lowest 2 bits of the fetched word:
    *   If bits are `11`: It is a standard 32-bit instruction. Advance `PC` by 4.
    *   Otherwise: It is a 16-bit compressed instruction. Advance `PC` by 2.
- [ ] Implement a **Decompression Translation Layer**: Translate 16-bit compressed instruction formats into their standard 32-bit RV64I counterparts *before* passing them to the decoder. This avoids rewriting execution logic.

### Phase 5: ELF Loading & Cycle Metering
- [ ] Integrate the `elf` crate to parse real `.elf` binaries generated by the CKB compiler.
- [ ] Map the binary sections (`.text`, `.rodata`, `.data`) directly into their designated executable/readable/writable VM memory pages.
- [ ] Implement deterministic cycle costs: increment the `cycles` counter on every instruction execution based on the real CKB-VM protocol spec (e.g., standard arithmetic = 1 cycle, memory loads = higher penalty).

---

## 4. Common Architectural Pitfalls to Avoid

*   **Rigid PC Increments:** Do not force `PC % 4 == 0`. Because CKB utilizes compressed instructions, the Program Counter regularly jumps on 2-byte boundaries (`PC % 2 == 0`).
*   **Ignoring Sign Extension:** Missing sign extension on 32-bit signed operations inside a 64-bit architecture (like `ADDW` or immediate shifts) will cause negative memory offsets to calculate as massive positive addresses, resulting in instant page faults.
*   **Shared State Dependencies:** Keep the VM state isolated. The only way data enters the VM memory must be explicitly through the initialized ELF structure or via verified `ECALL` handling boundaries.

---

## 5. Technical Context

*   **Target Core Architecture:** RV64IMC (Unprivileged User-Level ISA).
*   **Reference Standard:** *RISC-V Instruction Set Manual (Volume I)* & *Nervos RFC 0003 (CKB VM)*.
*   **Primary Tooling:** Rust (Stable), `ckb-riscv-gnu-toolchain`.