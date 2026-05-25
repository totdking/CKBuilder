use anyhow::{anyhow,Result};
pub struct CKbVm{
    registers: [u64;32], // enforce registers[0] hardwired to 0
    pc: u64, // program counter (where we are in the program)
    memory: Vec<u8> // the ram the program reads/writs
}

// fetch 4 bytes from mem at pc
pub fn fetch_4_bytes(memory: &[u8], pc: u64) -> Result<u32> {
    // it has to be from the memory address to mem_add pc + 3
    let mem_len = memory.len();
    if (pc + 3) >= mem_len as u64 {
        return Err(anyhow!("Pc index is out of bounds"));
    }
    let byte1 = memory[pc as usize] as u32;
    let byte2 = memory[(pc + 1) as usize] as u32;
    let byte3 = memory[(pc + 2) as usize] as u32;
    let byte4 = memory[(pc + 3) as usize] as u32;
    // risc v is little endian, smallest address holds the lsb
    // for [0-3] 0 is lsb, 3 is msb
    let ix = byte1 | (byte2 << 8) | (byte3 << 16) | (byte4 << 24);
    // or 
    // let ix = u32::from_le_bytes([byte1 as u8, byte2 as u8, byte3 as u8, byte4 as u8]);
    return Ok(ix);

}

// opcodes for the ckbVM
impl CKbVm{
    pub fn read_mem(&mut self) {
        let raw = fetch_4_bytes(&self.memory, self.pc).unwrap();
        let bit_type = raw & 0b11;
        if bit_type == 0b11 {
            // 32-bit ix
            self.pc += 4;
        }else {
            // compressed 16 bit ix
            self.pc += 2;
        }
    }
    pub fn execute(&mut self, bit: u32){
        self.registers[0] = 0;
        self.
    }
    pub fn add(){}
    pub fn add_i(){}
    pub fn jal(){}
    pub fn ecall(){}
}