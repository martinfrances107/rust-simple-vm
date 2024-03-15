use crate::register::Register;
use macros::VmInstruction;
use std::fmt;

/**
 * TYPE A
 * 1RRR LLLL LLLL LLLL
 * TYPE B
 * 0RRR SSSA AAAA DDDD
 * Add (A = 0)
 * [ assume register A = 1, B = 2 ]
 * Reg[D] = Reg[R] + Reg[S]
 * 1001 0100 0000 0001
 * Reg[0] is always = 0
 * Reg[A] = Reg[B] + Reg[Zero]
 * 1000 0100 0000 0001
 *
 * Stack (A = 0xf)
 * Push (D = 0)
 * 1001 1010 1111 0000
 * Pop (D = 1)
 * 1010 1010 1111 0001
 *
 * 1000 0000 0000 0000
 */

pub enum InstructionParseError {
    NoContent,
    Fail(String),
}

pub trait InstructionPart {
    fn as_mask(&self) -> u16;
    fn from_instruction(ins: u16) -> Self;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Literal7Bit {
    pub value: u8,
}

impl Literal7Bit {
    pub fn new(value: u8) -> Self {
        Self { value }
    }

    pub fn as_signed(&self) -> i8 {
        let sgn = (self.value & 0x40) >> 7;
        if sgn == 0 {
            (self.value & 0x3f) as i8
        } else {
            -((self.value & 0x3f) as i8)
        }
    }
}

impl fmt::Display for Literal7Bit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Literal10Bit {
    pub value: u16,
}

impl Literal10Bit {
    pub fn new(value: u16) -> Self {
        Self { value }
    }
}

impl fmt::Display for Literal10Bit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TestOp {
    Eq,
    Neq,
    Lt,
    Lte,
    BothZero,
    EitherNonZero,
    BothNonZero,
}

impl TryFrom<u16> for TestOp {
    type Error = String;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            x if x == TestOp::Eq as u16 => Ok(TestOp::Eq),
            x if x == TestOp::Neq as u16 => Ok(TestOp::Neq),
            x if x == TestOp::Lt as u16 => Ok(TestOp::Lt),
            x if x == TestOp::Lte as u16 => Ok(TestOp::Lte),
            x if x == TestOp::BothZero as u16 => Ok(TestOp::BothZero),
            x if x == TestOp::BothNonZero as u16 => Ok(TestOp::BothNonZero),
            x if x == TestOp::EitherNonZero as u16 => Ok(TestOp::EitherNonZero),
            _ => Err(format!("unknown test op value {}", value)),
        }
    }
}

impl InstructionPart for TestOp {
    fn as_mask(&self) -> u16 {
        (*self as u16) & 0xf
    }
    fn from_instruction(ins: u16) -> Self {
        TestOp::try_from(ins).unwrap()
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StackOp {
    Pop,
    Push,
    Peek,
    Swap,
    Dup,
    Rotate,
}

impl InstructionPart for StackOp {
    fn as_mask(&self) -> u16 {
        (*self as u16) & 0xf
    }

    fn from_instruction(ins: u16) -> Self {
        StackOp::try_from(ins).unwrap()
    }
}

impl TryFrom<u16> for StackOp {
    type Error = String;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            x if x == StackOp::Pop as u16 => Ok(StackOp::Pop),
            x if x == StackOp::Push as u16 => Ok(StackOp::Pop),
            x if x == StackOp::Peek as u16 => Ok(StackOp::Pop),
            x if x == StackOp::Swap as u16 => Ok(StackOp::Pop),
            x if x == StackOp::Dup as u16 => Ok(StackOp::Pop),
            x if x == StackOp::Rotate as u16 => Ok(StackOp::Pop),
            _ => Err(format!("unknown stack op value {}", value)),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Nibble {
    pub value: u8,
}

impl Nibble {
    pub fn new(value: u8) -> Self {
        Self { value }
    }
}

impl fmt::Display for Nibble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, VmInstruction, PartialEq, Eq)]
pub enum Instruction {
    #[opcode(0xff)]
    Imm(Register, u16),
    #[opcode(0x1)]
    Add(Register, Register, Register),
    #[opcode(0x2)]
    Sub(Register, Register, Register),
    #[opcode(0x3)]
    AddImm(Register, Literal7Bit),
    #[opcode(0x4)]
    AddImmSigned(Register, Literal7Bit),
    #[opcode(0x5)]
    ShiftLeft(Register, Register, Nibble),
    #[opcode(0x6)]
    ShiftRightLogical(Register, Register, Nibble),
    #[opcode(0x7)]
    ShiftRightArithmetic(Register, Register, Nibble),
    /*
    #[opcode(0x7)]
    Load(Register, Register, Register), // R0 = RAM[R1 | (R2<<16)]
    #[opcode(0x8)]
    Store(Register, Register, Register), // RAM[R1 | (R2<<16)] = R0
    #[opcode(0x9)]
    Test(Register, Register, TestOp),
    #[opcode(0xa)]
    AddIf(Register, Nibble),
    #[opcode(0xb)]
    Jump(Literal10Bit),
    #[opcode(0xc)]
    Stack(Register, Register, StackOp),
    #[opcode(0xd)]
    LoadStackOffset(Register, Register, Nibble),
    */
    #[opcode(0xe)]
    System(Register, Register, Nibble),
}

#[cfg(test)]
mod test {
    use super::Instruction::*;
    use super::*;
    use crate::register::Register::*;

    #[test]
    fn test_encodings() -> Result<(), String> {
        let ops = vec![
            Imm(M, 0x30),
            AddImm(C, Literal7Bit::new(0x20)),
            Add(C, B, A),
            Sub(PC, BP, SP),
            AddImmSigned(A, Literal7Bit::new(0x7)),
            ShiftLeft(M, BP, Nibble::new(0xe)),
            ShiftRightLogical(M, BP, Nibble::new(0xe)),
            ShiftRightArithmetic(M, BP, Nibble::new(0xe)),
            System(A, B, Nibble::new(0x3)),
        ];
        let encoded: Vec<_> = ops.iter().map(|x| x.encode_u16()).collect();
        for (l, r) in ops.iter().zip(encoded.iter()) {
            assert_eq!(*l, Instruction::try_from(*r)?);
        }
        Ok(())
    }
}
