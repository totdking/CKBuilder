use crate::instructions::decode::{
    get_funct3, get_imm_i, get_imm_j, get_imm_s, get_opcode, get_rd, get_rs1, get_rs2, sign_extend,
};
use anyhow::{Ok, Result, anyhow};

const JAL: u32 = 0b1101111; // 111
const ADD_I: u32 = 0b0010011; // 19
const ECALL: u32 = 0b1110011; // 115
const LOAD: u32 = 0b0000011; // 3
const STORE: u32 = 0b0100011; // 35

pub struct CkbVm {
    registers: [u64; 32], // enforce registers[0] hardwired to 0
    pub pc: u64,          // program counter (where we are in the program)
    memory: Vec<u8>,      // the ram the program reads/writs
}

// opcodes for the ckbVM
impl CkbVm {
    pub fn new(memory: Vec<u8>) -> Self {
        Self {
            registers: [0_u64; 32],
            pc: 0,
            memory,
        }
    }
    // for debugging / testing
    pub fn register(&self, idx: usize) -> u64 {
        self.registers[idx]
    }

    // for debugging and testing
    pub fn fetch_ix_at(&self, addr: usize) -> Result<u32> {
        if (addr + 4) > self.memory.len() {
            return Err(anyhow!("Address {:#?} out of bounds", &addr));
        }
        Ok(u32::from_le_bytes([
            self.memory[addr],
            self.memory[addr + 1],
            self.memory[addr + 2],
            self.memory[addr + 3],
        ]))
    }
    // fetch 4 bytes / 32 bits from mem at pc which is also ix or 32 bytes
    fn fetch_ix(&self) -> Result<u32> {
        self.fetch_ix_at(self.pc as usize)
    }

    // public function to be passed into implementation
    pub fn fetch(&self) -> Result<u32> {
        let raw = self.fetch_ix()?;
        Ok(raw)
    }

    /// Determines the type of jump whether a 2(16-bit compressed) or a 4(32 bit jump)
    fn get_ix_jump(&self) -> u32 {
        // get the program counter which would be the index we fetch memory from
        let mem_address = self.pc as usize;
        let first_byte = self.memory[mem_address];
        if first_byte & 0b11 == 0b11 {
            return 4;
        }
        return 2;
    }

    // Execute should be able to execute whatever ix passed in from add, jal, ECALL, ADD, load , store etc
    // This will be the entrypoint of the entire thing
    pub fn execute(&mut self, bit: u32) {
        self.registers[0] = 0; // We must make sure no matter what the registers[0] must be zero what i 
        // self.
    }

    pub fn branch(&mut self, bit: u32) -> Result<()> {
        let raw = bit;

        todo!()
    }

    // take N bytes from a register an write in to memory at "some address"
    pub fn store(&mut self, ix: u32) -> Result<()> {
        let raw = ix;
        let opcode = get_opcode(raw);
        if opcode != STORE {
            return Err(anyhow!("Invalid opcode for STORE"));
        }
        let rs1 = get_rs1(raw) as usize;
        let rs2 = get_rs2(raw) as usize;
        let funct3 = get_funct3(raw);
        let imm = get_imm_s(raw);

        // unlike load that has rd(destination register), store uses the registers[rs2] as the source where data would be carried from
        // and (registers[rs1] + imm) as the memory address to store the data in memory
        let value = self.registers[rs2];
        let mem_addr = (self.registers[rs1] as i64).wrapping_add(imm) as usize;
        match funct3 {
            0b000 => {
                /* SB - 1 byte */
                self.memory[mem_addr] = value as u8;
            }
            0b001 => {
                /* SH - 2 bytes */
                let val_bytes = (value as u16).to_le_bytes();
                self.memory[mem_addr] = val_bytes[0];
                self.memory[mem_addr + 1] = val_bytes[1];
            }
            0b010 => {
                /* SW - 4 bytes */
                let val_bytes = (value as u32).to_le_bytes();
                self.memory[mem_addr] = val_bytes[0];
                self.memory[mem_addr + 1] = val_bytes[1];
                self.memory[mem_addr + 2] = val_bytes[2];
                self.memory[mem_addr + 3] = val_bytes[3];
            }
            0b011 => {
                /* SD - 8 bytes */
                let val_bytes = (value as u64).to_le_bytes();
                self.memory[mem_addr] = val_bytes[0];
                self.memory[mem_addr + 1] = val_bytes[1];
                self.memory[mem_addr + 2] = val_bytes[2];
                self.memory[mem_addr + 3] = val_bytes[3];
                self.memory[mem_addr + 4] = val_bytes[4];
                self.memory[mem_addr + 5] = val_bytes[5];
                self.memory[mem_addr + 6] = val_bytes[6];
                self.memory[mem_addr + 7] = val_bytes[7];
            }
            _ => return Err(anyhow!("Unknown store funct3: {:#05b}", funct3)),
        }
        self.pc += self.get_ix_jump() as u64;
        Ok(())
    }

    // Go to some address in mem, grab N bytes and put the assembled value in a register
    pub fn load(&mut self, ix: u32) -> Result<()> {
        let raw = ix;
        let opcode = get_opcode(raw);
        if opcode != LOAD {
            return Err(anyhow!("Invalid opcode for load"));
        }
        let rd = get_rd(raw) as usize;
        let rs1 = get_rs1(raw);
        let imm = get_imm_i(raw);
        let funct3 = get_funct3(raw);
        // get the mem_addr
        let mem_addr = (self.registers[rs1 as usize] as i64).wrapping_add(imm) as usize;
        // sice we're writing from memory to register, we have to fill in the blank bytes with the intended sign
        match funct3 {
            0b000 => {
                /* LB */
                // 1 byte
                if rd != 0 {
                    let value = self.memory[mem_addr] as u64;
                    self.registers[rd] = sign_extend(value, 8) as u64;
                }
            }
            0b001 => {
                /* LH  */
                // 2 bytes
                if rd != 0 {
                    let value =
                        u16::from_le_bytes([self.memory[mem_addr], self.memory[mem_addr + 1]])
                            as u64;
                    self.registers[rd] = sign_extend(value, 16) as u64;
                }
            }
            0b010 => {
                /* LW  */
                // 4 bytes
                if rd != 0 {
                    let mut val_arr: [u8; 4] = [0_u8, 0_u8, 0_u8, 0_u8];
                    for i in 0..4 {
                        let arr_denom = self.memory[mem_addr + i];
                        val_arr[i] = arr_denom;
                    }
                    let value = u32::from_le_bytes(val_arr) as u64;
                    self.registers[rd] = sign_extend(value, 32) as u64;
                }
            }
            0b011 => {
                /* LD  */
                // 8 bytes
                if rd != 0 {
                    let mut val_arr: [u8; 8] = [0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8];
                    for i in 0..8 {
                        let arr_denom = self.memory[mem_addr + i];
                        val_arr[i] = arr_denom;
                    }
                    let value = u64::from_le_bytes(val_arr);
                    self.registers[rd] = value;
                }
            }
            0b100 => {
                /* LBU */
                // 1 byte unsigned
                if rd != 0 {
                    self.registers[rd] = self.memory[mem_addr] as u64;
                }
            }
            0b101 => {
                /* LHU */
                // 2 bytes unsigned
                if rd != 0 {
                    let value =
                        u16::from_le_bytes([self.memory[mem_addr], self.memory[mem_addr + 1]])
                            as u64;
                    self.registers[rd] = value;
                }
            }
            0b110 => {
                /* LWU */
                if rd != 0 {
                    let mut val_arr: [u8; 4] = [0_u8, 0_u8, 0_u8, 0_u8];
                    for i in 0..4 {
                        let arr_denom = self.memory[mem_addr + i];
                        val_arr[i] = arr_denom;
                    }
                    let value = u32::from_le_bytes(val_arr) as u64;
                    self.registers[rd] = value as u64;
                }
            }
            _ => return Err(anyhow!("Unknown load funct3: {:#05b}", funct3)),
        };
        self.pc += self.get_ix_jump() as u64;
        Ok(())
    }

    /// Environment Call
    /// This has 3 parameters: opcode(0-6), rd(7-11), funct3(12-14), rs1(15-19), funct12(20-31)
    // It does not need a destination register.
    // It is a breakpoint, it halts execution/signal a debug handler.
    pub fn ebreak(&mut self, ix: u32) -> Result<()> {
        // All the fields except the opcode are zeroed out
        let raw = ix;
        let opcode = raw & 0x7f;
        if opcode != ECALL {
            return Err(anyhow!("Not a valid EBREAK opcode"));
        }

        let rd = (raw >> 7) & 0b11111;
        let funct3 = (raw >> 12) & 0b111;
        let rs1 = (raw >> 15) & 0b11111;
        let funct12 = (raw >> 20) & 0xfff;

        if rd != 0 || funct3 != 0 || rs1 != 0 || funct12 != 1 {
            return Err(anyhow!("Not a valid EBREAk instruction"));
        }

        Err(anyhow!("EBREAK:debugger trap"))
    }

    /// Environment Call
    /// This has 3 parameters: opcode(0-6), rd(7-11), funct3(12-14), rs1(15-19), funct12(20-31)
    // It does not need a destination register.
    pub fn ecall(&mut self, ix: u32) -> Result<()> {
        // All the fields except the opcode are zeroed out
        let raw = ix;
        let opcode = raw & 0x7f;
        if opcode != ECALL {
            return Err(anyhow!("Not a valid ECALL opcode"));
        }

        let rd = (raw >> 7) & 0b11111;
        let funct3 = (raw >> 12) & 0b111;
        let rs1 = (raw >> 15) & 0b11111;
        let funct12 = (raw >> 20) & 0xfff;

        if rd != 0 || funct3 != 0 || rs1 != 0 || funct12 != 0 {
            return Err(anyhow!("One of the extra fields of ECALL is Non-Zero"));
        }
        // input goes through a0-a5(register[10] -> register[15]) and a7(register[17])
        // Output comes out through a0(register[10])
        let syscall_num = self.registers[17]; // a7
        // These are fixed list defined in the ckb rfc (RFC9, RFC34, RFC50)
        match syscall_num {
            // VM Version 1
            93 => { /* ckb_exit */ }
            2051 => { /* ckb_load_transaction */ }
            2052 => { /* ckb_load_script */ }
            2061 => { /* ckb_load_tx_hash */ }
            2062 => { /* ckb_load_script_hash */ }
            2071 => { /* ckb_load_cell */ }
            2072 => { /* ckb_load_header */ }
            2073 => { /* ckb_load_input */ }
            2074 => { /* ckb_load_witness */ }
            2081 => { /* ckb_load_cell_by_field */ }
            2082 => { /* ckb_load_header_by_field */ }
            2083 => { /* ckb_load_input_by_field */ }
            2091 => { /* ckb_load_cell_data_as_code */ }
            2092 => { /* ckb_load_cell_data */ }
            2177 => { /* ckb_debug */ }
            // VM Version 2
            2041 => { /* ckb_vm_version */ }
            2042 => { /* ckb_current_cycles */ }
            2043 => { /* ckb_exec */ }
            _ => return Err(anyhow!("Unknown syscall: {}", syscall_num)),
        }
        // save the result in the a0 : success(0) , fail(1)
        self.registers[10] = 0;
        // pc advanced
        self.pc += self.get_ix_jump() as u64;
        Ok(())
    }

    // add_immediate function
    // it is an I-type of ix with format : opcode(7), rd(5 bits), funct3(3 bits), rs1(5 bits), imm(12 bits)
    // 0-6, 7-11, 12-14, 15-19, 20-31
    pub fn add_i(&mut self, ix_bit: u32) -> Result<()> {
        // isolate all parts of it
        let raw = ix_bit;
        let opcode = raw & 0b1111111;
        // reject the opcode if its not the verified add_i
        if opcode != ADD_I {
            return Err(anyhow!("Not a valid ADD_I opcode"));
        }

        let rd = (raw >> 7) & 0b11111;
        let rs1 = (raw >> 15) & 0b11111;

        let funct3 = (raw >> 12) & 0b111;
        assert!(funct3 == 0b000);

        let signed_imm = get_imm_i(raw); // sign-extended 12-bit immediate
        if rd != 0 {
            self.registers[rd as usize] =
                self.registers[rs1 as usize].wrapping_add(signed_imm as u64);
        }
        let ix_size = self.get_ix_jump() as u64;
        self.pc += ix_size;
        Ok(())
    }

    /// JUMP and LINK
    /// Jal is a j-type ix format: opcode(7 bits) , rd(5 bits) , imm(20 bits)
    /// 0-6, 7-11, 12-19, 20, 21-30, 31
    pub fn jal(&mut self, ix_bit: u32) -> Result<()> {
        let raw = ix_bit;
        // isolating the opcode (0-6)
        let opcode = raw & 0b1111111;
        assert!(opcode == JAL, "Not a valid JAL instruction");
        // save the return address
        let ix_size = self.get_ix_jump() as u64;
        let save_addr = self.pc + ix_size;

        // isolate the rd (bit 7-11)
        // already registers[0] is already hardcoded to zero , but if
        // rd == 0 , it overwrites it in the self.registers[] below
        let rd = (raw >> 7) & 0b11111;
        // Handled the occurence of rd == 0
        // save return address in destination
        if rd != 0 {
            self.registers[rd as usize] = save_addr;
        }

        let signed = get_imm_j(raw); // sign-extended 21-bit J-type immediate
        self.pc = self.pc.wrapping_add_signed(signed);
        Ok(())
    }
}
