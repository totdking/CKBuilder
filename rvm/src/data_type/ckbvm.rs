use anyhow::{Ok, Result, anyhow};
const JAL: u32 = 0x1101111;
// const ADD: u32
// const ADD_I: u32
// const SUB: u32

pub struct CkbVm{
    pub registers: [u64;32], // enforce registers[0] hardwired to 0
    pub pc: u64, // program counter (where we are in the program)
    memory: Vec<u8> // the ram the program reads/writs
}



// opcodes for the ckbVM
impl CkbVm{
    pub fn new(memory: Vec<u8>) -> Self{
        Self { 
            registers: [0_u64;32], 
            pc: 0, 
            memory
        }
    }

    // fetch 4 bytes from mem at pc
    fn fetch_4_bytes(&self, pc: u64) -> Result<u32> {
        // it has to be from the memory address to mem_add pc + 3
        let mem_len = self.memory.len();
        if (pc + 3) >= mem_len as u64 {
            return Err(anyhow!("Pc index is out of bounds"));
        }
        let byte1 = self.memory[pc as usize] as u32;
        let byte2 = self.memory[(pc + 1) as usize] as u32;
        let byte3 = self.memory[(pc + 2) as usize] as u32;
        let byte4 = self.memory[(pc + 3) as usize] as u32;
        // risc v is little endian, smallest address holds the lsb
        // for [0-3] 0 is lsb, 3 is msb
        let ix = byte1 | (byte2 << 8) | (byte3 << 16) | (byte4 << 24);
        // or 
        // let ix = u32::from_le_bytes([byte1 as u8, byte2 as u8, byte3 as u8, byte4 as u8]);
        return Ok(ix);

    }

    pub fn fetch (&self) -> Result<u32>{
        let raw = self.fetch_4_bytes(self.pc)?;
        Ok(raw)
    }
    // Execute should be able to execute whatever ix passed in from add, jal, load, etc
    pub fn execute(&mut self, bit: u32){
        self.registers[0] = 0; // We must make sure no matter what the registers[0] must be zero what i 
        // self.
    }
    pub fn add(){}
    pub fn add_i(){}
    pub fn ecall(){}
    pub fn load(){}

    /// JUMP and LINK
    /// Jal is a j-type ix format: opcode(7 bits) , rd(5 bits) , imm(20 bits)
    /// 0-6, 7-11, 12-19, 20, 21-30, 31
    pub fn jal (&mut self, ix_bit: u32) -> Result<()> {
        let raw = ix_bit;
        // isolatiing the opcode (0-6)
        let opcode = raw & 0b1111111;
        assert!(opcode == JAL, "Not a valid JAL instruction");
        // save the return address
        let save_addr = self.pc + 4;
        
        // isolate the rd (bit 7-11)
        // already registers[0] is already hardcoded to zero , but if 
        // rd == 0 , it overwrites it in the self.registers[] below
        let rd = (raw >> 7) & 0b11111;
        // Handled the occurence of rd == 0
        if rd != 0 {
            self.registers[rd as usize] = save_addr;    
        }

        // isolate the imm / immediate (12-31)
        // let imm = (raw >> 12) & 0b11111111111111111111; 
        let imm_1_10 = (raw >> 21) & 0b1111111111; // gotten from bit 21-30
        let imm_11 = (raw >> 20) & 0b1; // gotten from bit 20
        let imm_12_19 = (raw >> 12) &0b11111111; // gotten from bit 12-19
        let imm_20 = (raw >> 31) & 0b1; // gotten from bit 31

        // The final immediate is 21 bits
        let final_imm = imm_12_19 << 12 | imm_11 << 11 | imm_1_10 << 1 | imm_20 << 20;

        // if bit 20 is 1(-ve) since it will still be that same bit 20 at the msb
        // the arithmetic right shift fills the holes with copies of the msb which
        // is 1 from the check above
        // right shift on signed integers behave like this
        let signed = ((final_imm as i64) << 43) >> 43;

        self.pc = self.pc.wrapping_add_signed(signed);
        Ok(())
    }
}