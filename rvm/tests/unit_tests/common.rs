use rvm::data_type::ckbvm::CkbVm;

/// Creates a Vm instance but with instructions pre-loaded
///
/// For tests purposes
pub fn vm_with_ix(ix: u32) -> CkbVm {
    let mut mem = vec![0b11u8; 4096];
    mem[0..4].copy_from_slice(&ix.to_le_bytes());
    CkbVm::new(mem)
}
