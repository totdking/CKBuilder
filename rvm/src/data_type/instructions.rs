use anyhow::{Ok, anyhow, Result};

use crate::instructions::decode::get_opcode;

enum IxType{
    Jal, Add, AddI, Ecall, Load
}

/// These are opcode groups
#[derive(Debug)]
pub enum Instruction{
    /// R-type (7opcode, 5rd, 3funct3, 5rs1, 5rs2, 7funct7)
    Register,
    /// I-type (7, 5, 3, 5, 12imm)
    OppImm,
    /// S-type (7, 5imm, 3, 5, 5rs2, 7imm)
    Store,
    /// B-type (7, 5imm, 3, 5, 5, 7) or branch type
    StoreCond,
    /// U-type (7, 5rd, 20imm)
    UpperImm,
    /// J-type (7, 5, 20imm)
    Jump,
    /// ECALL and EBREAK syscalls
    System,
    /// Load(LW, LB)
    Load
}

pub fn dispatch(raw: u32) -> Result<Instruction> {
    let opcode = get_opcode(raw);
    match opcode {
        0b0110011 => Ok(Instruction::Register),   // R-type ADD, SUB(Add + -ve int), AND, OR...
        0b0010011 => Ok(Instruction::OppImm),     // I-type ADDI, ANDI, ORI...
        0b0100011 => Ok(Instruction::Store),      // S-type SW, SB, SH...
        0b1100011 => Ok(Instruction::StoreCond),  // B-type BEQ, BNE, BLT...
        0b0110111 => Ok(Instruction::UpperImm),   // U-type LUI
        0b1101111 => Ok(Instruction::Jump),       // J-type JAL
        0b1110011 => Ok(Instruction::System),      // This is the 
        0b0000011 => Ok(Instruction::Load),
        _ => Err(anyhow!("Unknown opcode: {:#09b}", opcode)),
    }
}

impl Instruction {
    pub fn dispatcher (&self) -> Self{
        todo!()
    }
}

