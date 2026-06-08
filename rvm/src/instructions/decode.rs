//! This file helps with the getting of fields of the R-type Ix
pub fn get_opcode(bit: u32) -> u32{
    bit & 0b1111111
}

/// accepts 32 bits 0x00A58533 and isolates bits 7–11 to extract the destination register (rd).
pub fn get_rd(bit: u32) -> u32{
    // reg_dest = (ix >> 7) & b11111
    // shift by 7 bits
    let shift = bit >> 7;
    // mask the last 5 bits
    let rd = shift & 0b11111;
    return rd;
}

pub fn get_funct3(bit: u32) -> u32{
    (bit >> 12) & 0b111
}

pub fn get_rs1(bit: u32) -> u32{
    (bit >> 15) & 0b11111
}
pub fn get_rs2(bit: u32) -> u32{
    (bit >> 20) & 0b11111
}
pub fn get_funct7(bit: u32) -> u32{
    (bit >> 25) & 0b1111111
}

#[test]
fn test_decoding(){
    let a = 0x00A58533; // 0b 00000000(fucnt7 = 0) 01010(rs2 = 10) 01011(rs1 = 11) 000(sub op selector = FUNCT 3 0/ADD) 01010(DEST REG = 10)  0110011(IX type)

    assert!(get_rd(a) == 0b01010);
    assert_eq!(get_opcode(a),  0b0110011); // 51, R-type
    assert_eq!(get_rd(a),      0b01010);   // 10
    assert_eq!(get_funct3(a),  0b000);     // ADD
    assert_eq!(get_rs1(a),     0b01011);   // 11
    assert_eq!(get_rs2(a),     0b01010);   // 10
    assert_eq!(get_funct7(a),  0b0000000); // 0

}