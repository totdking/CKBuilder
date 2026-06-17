//! Decoding helpers for all RISC-V instruction fields and immediate types.
//!
//! Spec ref: riscv-unprivileged.pdf §2.2–2.3 (pages 25–27).
//! Key invariant from the spec: bit 31 of the raw instruction word is ALWAYS
//! the sign bit for every immediate type, regardless of format.

// ── Field extractors (common to all formats) ────────────────────────────────

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

// ── Sign extension ───────────────────────────────────────────────────────────

/// Propagate the MSB of a `bit_width`-wide value across all upper bits to 64.
///
/// Spec ref §2.3 p.27: "Sign extension is one of the most critical operations
/// on immediates … the sign bit for all immediates is always held in bit 31."
///
/// Mechanism: shift the sign bit up to bit 63, then arithmetic-right-shift
/// back — Rust's `>>` on `i64` is arithmetic, so it fills with copies of
/// the sign bit.
pub fn sign_extend(value: u64, bit_width: u32) -> i64 {
    let shift = 64 - bit_width;
    ((value as i64) << shift) >> shift
}

// ── Immediate extractors (one per format) ────────────────────────────────────
// Each reassembles the scrambled instruction bits into a logical immediate
// value then sign-extends it. Bit layouts from Figure 1 (spec p.27).

/// I-type: inst[31:20] → imm[11:0]  (12-bit, always contiguous)
pub fn get_imm_i(raw: u32) -> i64 {
    let imm = (raw >> 20) & 0xFFF;
    sign_extend(imm as u64, 12)
}

/// S-type: inst[31:25] → imm[11:5], inst[11:7] → imm[4:0]
pub fn get_imm_s(raw: u32) -> i64 {
    let imm_4_0  = (raw >> 7)  & 0b11111;
    let imm_11_5 = (raw >> 25) & 0b1111111;
    let imm = (imm_11_5 << 5) | imm_4_0;
    // for s-type, inst[31] is imm[11] which is the 12th value
    sign_extend(imm as u64, 12)
}

/// B-type: scrambled 13-bit immediate, imm[0] is always 0 (branch offsets
/// are multiples of 2). inst[31]→imm[12], inst[7]→imm[11],
/// inst[30:25]→imm[10:5], inst[11:8]→imm[4:1].
pub fn get_imm_b(raw: u32) -> i64 {
    let imm_4_1  = (raw >> 8)  & 0b1111;
    let imm_10_5 = (raw >> 25) & 0b111111;
    let imm_11   = (raw >> 7)  & 0b1;
    let imm_12   = (raw >> 31) & 0b1;
    let imm = (imm_12 << 12) | (imm_11 << 11) | (imm_10_5 << 5) | (imm_4_1 << 1);
    // inst[31] is imm[12] which is the 13th value
    sign_extend(imm as u64, 13)
}

/// U-type: inst[31:12] → imm[31:12], lower 12 bits are zero (LUI/AUIPC).
/// 32-bit value; sign-extend to 64 for RV64.
pub fn get_imm_u(raw: u32) -> i64 {
    let imm = raw & 0xFFFFF000;
    // inst[31] == imm[31] which is the 32nd value
    sign_extend(imm as u64, 32)
}

/// J-type: scrambled 21-bit immediate, imm[0] always 0 (jump offsets are
/// multiples of 2). inst[31]→imm[20], inst[19:12]→imm[19:12],
/// inst[20]→imm[11], inst[30:21]→imm[10:1].
pub fn get_imm_j(raw: u32) -> i64 {
    let imm_10_1  = (raw >> 21) & 0b1111111111;
    let imm_11    = (raw >> 20) & 0b1;
    let imm_19_12 = (raw >> 12) & 0b11111111;
    let imm_20    = (raw >> 31) & 0b1;
    let imm = (imm_20 << 20) | (imm_19_12 << 12) | (imm_11 << 11) | (imm_10_1 << 1);
    // inst[31] is imm[20] which is the 21st value
    sign_extend(imm as u64, 21)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

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

#[test]
fn test_sign_extend_positive() {
    // 5 fits in 12 bits with MSB=0 → stays positive
    assert_eq!(sign_extend(5, 12), 5_i64);
}

#[test]
fn test_sign_extend_negative() {
    // 0xFFF is -1 in 12-bit two's complement → all 64 bits become 1
    assert_eq!(sign_extend(0xFFF, 12), -1_i64);
    // 0x800 is the most-negative 12-bit value (-2048)
    assert_eq!(sign_extend(0x800, 12), -2048_i64);
}

#[test]
fn test_get_imm_i_positive() {
    // ADDI x1, x0, 5  →  0x00500093
    // inst[31:20] = 0x005 = 5
    assert_eq!(get_imm_i(0x00500093), 5);
}

#[test]
fn test_get_imm_i_negative() {
    // ADDI x1, x0, -1  →  0xFFF00093
    // inst[31:20] = 0xFFF → sign-extended = -1
    assert_eq!(get_imm_i(0xFFF00093), -1);
}

#[test]
fn test_get_imm_j_positive() {
    // JAL x1, 8  →  0x008000EF
    // imm[20:1] encodes +8 (imm[3]=1, rest 0)
    assert_eq!(get_imm_j(0x008000EF), 8);
}

#[test]
fn test_get_imm_b_positive() {
    // BEQ x0, x0, 8  → a standard forward branch by 8 bytes
    // Encoding: imm=8 → imm[4:1]=0b0100, all others 0
    // inst: 0b 0_000000_00000_00000_000_0100_0_1100011 = 0x00000463
    assert_eq!(get_imm_b(0x00000463), 8);
}

#[test]
fn test_get_imm_s() {
    // SW x1, 4(x2)  →  0x00112223
    // S-type imm = 4: imm[4:0]=0b00100, imm[11:5]=0b0000000
    assert_eq!(get_imm_s(0x00112223), 4);
}

#[test]
fn test_get_imm_u() {
    // LUI x1, 1  →  0x000010B7
    // U-type imm = 1 << 12 = 0x1000
    assert_eq!(get_imm_u(0x000010B7), 0x1000);
}