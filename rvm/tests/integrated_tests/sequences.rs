use rvm::data_type::ckbvm::CkbVm;

fn build_mem(instructions: &[u32]) -> Vec<u8> {
    let mut mem = vec![0u8; 4096];
    for (i, &ix) in instructions.iter().enumerate() {
        let offset = i * 4;
        mem[offset..offset + 4].copy_from_slice(&ix.to_le_bytes());
    }
    mem
}

// Execute a slice of (raw_ix, method) pairs in order, fetching each instruction
// at the current PC so the sequence mirrors real execution flow.
macro_rules! run_seq {
    ($vm:ident, [ $( ($ix:expr, $method:ident) ),* $(,)? ]) => {
        $(
            {
                let raw = $vm.fetch_ix_at($vm.pc as usize).unwrap();
                assert_eq!(raw, $ix, "unexpected instruction at pc={}", $vm.pc);
                $vm.$method(raw).unwrap();
            }
        )*
    };
}

#[test]
fn addi_addi_leaves_correct_register_state() {
    // ADDI x1, x0, 5   → x1 = 5
    // ADDI x2, x1, 10  → x2 = 15
    let mem = build_mem(&[0x00500093, 0x00A08113]);
    let mut vm = CkbVm::new(mem);

    let ix1 = vm.fetch_ix_at(0).unwrap();
    vm.add_i(ix1).unwrap();

    let ix2 = vm.fetch_ix_at(vm.pc as usize).unwrap();
    vm.add_i(ix2).unwrap();

    assert_eq!(vm.register(1), 5);
    assert_eq!(vm.register(2), 15);
    assert_eq!(vm.pc, 8);
}

#[test]
fn jal_jumps_over_instruction() {
    // byte 0:  ADDI x1, x0, 5
    // byte 4:  JAL  x2, 8      → skips byte 8, lands at byte 12
    // byte 8:  ADDI x1, x0, 99 → should be skipped
    // byte 12: ADDI x3, x0, 42 → should execute
    let mem = build_mem(&[0x00500093, 0x0080016F, 0x06300093, 0x02A00193]);
    let mut vm = CkbVm::new(mem);

    let ix1 = vm.fetch_ix_at(vm.pc as usize).unwrap();
    vm.add_i(ix1).unwrap();

    let ix2 = vm.fetch_ix_at(vm.pc as usize).unwrap();
    vm.jal(ix2).unwrap();

    let ix3 = vm.fetch_ix_at(vm.pc as usize).unwrap();
    vm.add_i(ix3).unwrap();

    assert_eq!(vm.register(1), 5); // not overwritten by skipped instruction
    assert_eq!(vm.register(3), 42); // set by instruction at byte 12
    assert_eq!(vm.pc, 16);
}

#[test]
fn ecall_dispatches_after_addi_sets_a7() {
    // ADDI x17, x0, 93  → sets a7 = 93 (ckb_exit syscall)
    // ECALL             → dispatches to ckb_exit arm (succeeds)
    let mem = build_mem(&[0x05D00893, 0x00000073]);
    let mut vm = CkbVm::new(mem);

    let ix1 = vm.fetch_ix_at(vm.pc as usize).unwrap();
    vm.add_i(ix1).unwrap();
    assert_eq!(vm.register(17), 93);

    let ix2 = vm.fetch_ix_at(vm.pc as usize).unwrap();
    vm.ecall(ix2).unwrap();
}

// ── Load / Store sequences ───────────────────────────────────────────────────
//
// Instruction encodings used below (all offsets relative to x0=0):
//   0x02A00093  ADDI x1, x0, 42
//   0x0C100423  SB   x1, 200(x0)
//   0x0C804103  LBU  x2, 200(x0)
//   0x1F400093  ADDI x1, x0, 500
//   0x0C102423  SW   x1, 200(x0)
//   0x0C806103  LWU  x2, 200(x0)
//   0x0FF00093  ADDI x1, x0, 255
//   0x0C800103  LB   x2, 200(x0)   (signed, so 0xFF → -1)
//   0x04200093  ADDI x1, x0, 66
//   0x03700113  ADDI x2, x0, 55
//   0x0C200623  SB   x2, 204(x0)
//   0x0C804183  LBU  x3, 200(x0)
//   0x0CC04203  LBU  x4, 204(x0)

#[test]
fn addi_store_load_byte_roundtrip() {
    // ADDI x1, x0, 42  → x1 = 42
    // SB   x1, 200(x0) → mem[200] = 42
    // LBU  x2, 200(x0) → x2 = 42
    let mem = build_mem(&[0x02A00093, 0x0C100423, 0x0C804103]);
    let mut vm = CkbVm::new(mem);

    run_seq!(
        vm,
        [(0x02A00093, add_i), (0x0C100423, store), (0x0C804103, load),]
    );

    assert_eq!(vm.register(1), 42);
    assert_eq!(vm.register(2), 42);
    assert_eq!(vm.pc, 12);
}

#[test]
fn addi_store_load_word_roundtrip() {
    // ADDI x1, x0, 500  → x1 = 500
    // SW   x1, 200(x0)  → mem[200..203] = 500 as 32-bit LE
    // LWU  x2, 200(x0)  → x2 = 500
    let mem = build_mem(&[0x1F400093, 0x0C102423, 0x0C806103]);
    let mut vm = CkbVm::new(mem);

    run_seq!(
        vm,
        [(0x1F400093, add_i), (0x0C102423, store), (0x0C806103, load),]
    );

    assert_eq!(vm.register(1), 500);
    assert_eq!(vm.register(2), 500);
    assert_eq!(vm.pc, 12);
}

#[test]
fn store_byte_load_signed_sign_extends() {
    // ADDI x1, x0, 255 → x1 = 0xFF
    // SB   x1, 200(x0) → mem[200] = 0xFF
    // LB   x2, 200(x0) → x2 = sign_extend(0xFF, 8) = -1
    let mem = build_mem(&[0x0FF00093, 0x0C100423, 0x0C800103]);
    let mut vm = CkbVm::new(mem);

    run_seq!(
        vm,
        [(0x0FF00093, add_i), (0x0C100423, store), (0x0C800103, load),]
    );

    assert_eq!(vm.register(2) as i64, -1);
    assert_eq!(vm.pc, 12);
}

#[test]
fn two_independent_stores_then_two_loads() {
    // ADDI x1, x0, 66  → x1 = 66
    // ADDI x2, x0, 55  → x2 = 55
    // SB   x1, 200(x0) → mem[200] = 66
    // SB   x2, 204(x0) → mem[204] = 55
    // LBU  x3, 200(x0) → x3 = 66
    // LBU  x4, 204(x0) → x4 = 55
    let mem = build_mem(&[
        0x04200093, // ADDI x1, x0, 66
        0x03700113, // ADDI x2, x0, 55
        0x0C100423, // SB   x1, 200(x0)
        0x0C200623, // SB   x2, 204(x0)
        0x0C804183, // LBU  x3, 200(x0)
        0x0CC04203, // LBU  x4, 204(x0)
    ]);
    let mut vm = CkbVm::new(mem);

    run_seq!(
        vm,
        [
            (0x04200093, add_i),
            (0x03700113, add_i),
            (0x0C100423, store),
            (0x0C200623, store),
            (0x0C804183, load),
            (0x0CC04203, load),
        ]
    );

    assert_eq!(vm.register(3), 66);
    assert_eq!(vm.register(4), 55);
    assert_eq!(vm.pc, 24);
}
