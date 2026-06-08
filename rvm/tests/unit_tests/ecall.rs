use super::common::vm_with_ix;

#[test]
fn ecall_invalid_opcode_errors() {
    let ix: u32 = 0x00500093; // ADDI encoding, not ECALL
    let mut vm = vm_with_ix(ix);
    assert!(vm.ecall(ix).is_err());
}

#[test]
fn ecall_nonzero_fields_rejected() {
    // valid ECALL opcode (0x73) but rs1 != 0 (bit 15 set)
    let ix: u32 = 0x00008073;
    let mut vm = vm_with_ix(ix);
    assert!(vm.ecall(ix).is_err());
}

#[test]
fn ecall_unknown_syscall_errors() {
    // valid ECALL encoding, but registers[17] = 0 which is not a CKB syscall
    let ix: u32 = 0x00000073;
    let mut vm = vm_with_ix(ix);
    assert!(vm.ecall(ix).is_err());
}
