use super::common::vm_with_ix;
use rvm::data_type::ckbvm::CkbVm;

#[test]
fn add_i_stores_result_in_rd() {
    // ADDI x1, x0, 5 — x0 is always 0, so x1 = 5
    let ix: u32 = 0x00500093;
    let mut vm = vm_with_ix(ix);
    vm.add_i(ix).unwrap();
    assert_eq!(vm.register(1), 5);
}

#[test]
fn add_i_rd_zero_is_never_written() {
    // ADDI x0, x0, 5 — rd=0, register[0] must stay 0
    let ix: u32 = 0x00500013;
    let mut vm = vm_with_ix(ix);
    vm.add_i(ix).unwrap();
    assert_eq!(vm.register(0), 0);
}

#[test]
fn add_i_adds_to_rs1() {
    // ADDI x1, x0, 5 then ADDI x2, x1, 10 — x2 should be 15
    let ix1: u32 = 0x00500093; // ADDI x1, x0, 5
    let ix2: u32 = 0x00A08113; // ADDI x2, x1, 10
    let mut mem = vec![0b11u8; 4096];
    mem[0..4].copy_from_slice(&ix1.to_le_bytes());
    mem[4..8].copy_from_slice(&ix2.to_le_bytes());
    let mut vm = CkbVm::new(mem);
    vm.add_i(ix1).unwrap();
    vm.add_i(ix2).unwrap();
    assert_eq!(vm.register(2), 15);
}

#[test]
fn add_i_pc_advances_by_4() {
    let ix: u32 = 0x00500093;
    let mut vm = vm_with_ix(ix);
    vm.add_i(ix).unwrap();
    assert_eq!(vm.pc, 4);
}

#[test]
fn add_i_wrong_opcode_errors() {
    let ix: u32 = 0x008000EF; // JAL encoding, not ADDI
    let mut vm = vm_with_ix(ix);
    assert!(vm.add_i(ix).is_err());
}
