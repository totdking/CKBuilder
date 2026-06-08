use rvm::data_type::ckbvm::CkbVm;

fn build_mem(instructions: &[u32]) -> Vec<u8> {
    let mut mem = vec![0u8; 4096];
    for (i, &ix) in instructions.iter().enumerate() {
        let offset = i * 4;
        mem[offset..offset + 4].copy_from_slice(&ix.to_le_bytes());
    }
    mem
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

    assert_eq!(vm.register(1), 5);   // not overwritten by skipped instruction
    assert_eq!(vm.register(3), 42);  // set by instruction at byte 12
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
