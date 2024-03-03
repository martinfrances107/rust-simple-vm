use std::fmt;
use std::str::FromStr;

use crate::register::Register;
use macros::VmInstruction;

/**
 * TYPE A
 * 0RRR LLLL LLLL LLLL
 * TYPE B
 * 1RRR SSSA AAAA DDDD
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

pub struct Literal7Bit {
    value: u8, 
}

impl InstructionPart for Literal7Bit {
    fn as_mask(&self) -> u16 {
        ((self.value&0xf) as u16) | ((self.value&0xe00) as u16)
    }
    fn from_instruction(ins: u16) -> Self {
        Self {
            value: ((ins&0xf) as u8) | (((ins&0xe00)>>5) as u8),
        }
    }
}

pub struct Literal10Bit {
    value: u16,
}

impl InstructionPart for Literal10Bit {
    fn as_mask(&self) -> u16 {
        ((self.value&0xf) as u16) | ((self.value&0xe00) as u16) | ((self.value&0x7000) as u16)
    }
    fn from_instruction(ins: u16) -> Self {
        Self {
            value: ((ins&0xf) as u16) | (((ins&0xe00)>>5) as u16) | (((ins&0x7000)>>5) as u16),
        }
    }

}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
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
        (self as u16)&0xf
    }
    fn from_instruction(ins: u16) -> Self {
        TestOp::try_from(ins).unwrap()
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
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
        (self as u16)&0xf
    }
}

pub struct Nibble {
    value: u8,
}

impl InstructionPart for Nibble {
    fn as_mask(&self) -> u16 {
        (self.value as u16)&0xf
    }
}

#[derive(Debug, VmInstruction)]
pub enum Instruction {
    #[opcode(0xff)]
    Imm(Register, u16),
    #[opcode(0x0)]
    Add(Register, Register, Register),
    #[opcode(0x1)]
    Sub(Register, Register, Register),
    #[opcode(0x2)]
    AddImm(Register, Literal7Bit),
    /*
    #[opcode(0x3)]
    AddImmSigned(Register, Literal7Bit),
    #[opcode(0x4)]
    ShiftLeft(Register, Register, Nibble),
    #[opcode(0x5)]
    ShiftRightLogical(Register, Register, Nibble),
    #[opcode(0x6)]
    ShiftRightArithmetic(Register, Register, Nibble),

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
    #[opcode(0xe)]
    System(Register, Register, Nibble),
    */
}

#[cfg(test)]
mod test {
    use super::Instruction::*;
    use super::*;

    #[test]
    fn test_encodings() {
        assert_eq!(SubStack.encode_u16(), 0x22 as u16);
        assert_eq!(Push(0x5).encode_u16(), 0x0501 as u16);
        assert_eq!(
            AddRegister(Register::B, Register::BP).encode_u16(),
            0x6121 as u16
        );
    }
}
