use rvm::data_type::ckbvm::CkbVm;

#[test]
fn fetch_ix_at_assembles_little_endian() {
    let mem = vec![0x93u8, 0x00, 0x50, 0x00]; // ADDI x1, x0, 5 = 0x00500093
    let vm = CkbVm::new(mem);
    assert_eq!(vm.fetch_ix_at(0).unwrap(), 0x00500093);
}

#[test]
fn fetch_ix_at_out_of_bounds_errors() {
    let vm = CkbVm::new(vec![0x93, 0x00, 0x50]); // only 3 bytes
    assert!(vm.fetch_ix_at(0).is_err());
}

#[test]
fn fetch_ix_at_offset() {
    let mut mem = vec![0u8; 8];
    mem[4..8].copy_from_slice(&0x00500093u32.to_le_bytes());
    let vm = CkbVm::new(mem);
    assert_eq!(vm.fetch_ix_at(4).unwrap(), 0x00500093);
}
