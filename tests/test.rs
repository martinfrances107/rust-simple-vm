use simplevm::Instruction::*;
use simplevm::Register::*;
use simplevm::*;

mod common;
use common::*;

macro_rules! test_set {
    ($vm:ident, $($prog:expr),+) => {
        run(&mut $vm, &[$($prog),*, System(Zero, Zero, Nibble::new(SIGHALT))])?;
        assert_flag_set!($vm, Flag::Compare);
    }
}

macro_rules! test_unset {
    ($vm:ident, $($prog:expr),+) => {
        run(&mut $vm, &[$($prog),*, System(Zero, Zero, Nibble::new(SIGHALT))])?;
        assert_flag_unset!($vm, Flag::Compare);
    }
}

#[test]
fn test_eq() -> Result<(), String> {
    let mut vm = Machine::new(1024 * 4);
    test_unset!(vm, Imm(A, 123), Imm(B, 567), Test(A, B, TestOp::Eq));
    test_set!(vm, Imm(A, 444), Imm(B, 444), Test(A, B, TestOp::Eq));
    Ok(())
}

#[test]
fn test_neq() -> Result<(), String> {
    let mut vm = Machine::new(1024 * 4);
    test_set!(vm, Imm(A, 123), Imm(B, 567), Test(A, B, TestOp::Neq));
    test_unset!(vm, Imm(A, 444), Imm(B, 444), Test(A, B, TestOp::Neq));
    Ok(())
}

#[test]
fn test_lt() -> Result<(), String> {
    let mut vm = Machine::new(1024 * 4);
    test_set!(vm, Imm(A, 44), Imm(B, 55), Test(A, B, TestOp::Lt));
    test_unset!(vm, Imm(A, 88), Imm(B, 44), Test(A, B, TestOp::Lt));
    test_set!(vm, Imm(A, 55), Imm(B, 55), Test(A, B, TestOp::Lte));
    test_unset!(vm, Imm(A, 88), Imm(B, 44), Test(A, B, TestOp::Lte));
    Ok(())
}

#[test]
fn test_gt() -> Result<(), String> {
    let mut vm = Machine::new(1024 * 4);
    test_unset!(vm, Imm(A, 44), Imm(B, 55), Test(A, B, TestOp::Gt));
    test_set!(vm, Imm(A, 88), Imm(B, 44), Test(A, B, TestOp::Gt));
    test_set!(vm, Imm(A, 55), Imm(B, 55), Test(A, B, TestOp::Gte));
    test_unset!(vm, Imm(A, 44), Imm(B, 88), Test(A, B, TestOp::Gte));
    Ok(())
}

#[test]
fn test_both_zero() -> Result<(), String> {
    let mut vm = Machine::new(1024 * 4);
    test_unset!(vm, Imm(A, 44), Test(A, B, TestOp::BothZero));
    test_set!(vm, Test(A, B, TestOp::BothZero));
    test_set!(vm, Test(Zero, Zero, TestOp::BothZero));
    Ok(())
}

#[test]
fn test_either_nonzero() -> Result<(), String> {
    let mut vm = Machine::new(1024 * 4);
    test_set!(vm, Imm(A, 44), Test(A, B, TestOp::EitherNonZero));
    test_unset!(vm, Test(A, B, TestOp::EitherNonZero));
    test_unset!(vm, Test(Zero, Zero, TestOp::EitherNonZero));
    Ok(())
}

#[test]
fn test_both_nonzero() -> Result<(), String> {
    let mut vm = Machine::new(1024 * 4);
    test_unset!(vm, Imm(A, 44), Test(A, B, TestOp::BothNonZero));
    test_unset!(vm, Test(Zero, Zero, TestOp::BothNonZero));
    test_set!(vm, Imm(A, 1), Imm(B, 2), Test(A, B, TestOp::BothNonZero));
    Ok(())
}
