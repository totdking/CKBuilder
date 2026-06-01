mod instructions;
mod data_type;
use data_type::ckbvm::{CkbVm};

fn main() {
    println!("Hello, world!");
    // 4mb buffer for ckb (CKB ACTUALLY HAS A 64 MB LIMIT)
    let mem = vec![1u8; 4 * 1024 * 1024];
    let mock_ckbvm = CkbVm::new(mem);

    let first_4_bytes = mock_ckbvm.fetch().unwrap();
    println!("first 4 bytes in hex {:#10x}", first_4_bytes);
    println!("first 4 bytes in bin {:#034b}", first_4_bytes)
    
}
