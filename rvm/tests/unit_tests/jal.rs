use super::common::vm_with_ix;

#[test]
fn jal_saves_return_address_in_rd() {
    // JAL x1, 8 — at pc=0, return address = 0 + 4 = 4
    let ix: u32 = 0x008000EF;
    let mut vm = vm_with_ix(ix);
    vm.jal(ix).unwrap();
    assert_eq!(vm.register(1), 4);
}

#[test]
fn jal_pc_jumps_to_target() {
    // JAL x1, 8 — from pc=0, target = 0 + 8 = 8
    let ix: u32 = 0x008000EF;
    let mut vm = vm_with_ix(ix);
    vm.jal(ix).unwrap();
    assert_eq!(vm.pc, 8);
}

#[test]
fn jal_rd_zero_does_not_save_return_address() {
    // JAL x0, 8 — rd=0, register[0] must stay 0
    let ix: u32 = 0x0080006F;
    let mut vm = vm_with_ix(ix);
    vm.jal(ix).unwrap();
    assert_eq!(vm.register(0), 0);
}
