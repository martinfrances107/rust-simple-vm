use simplevm::{Instruction, Register, Literal12Bit, Literal7Bit, Literal10Bit};
use std::fmt;

use crate::compile::context::Context;
use crate::compile::error::CompilerError;
use crate::compile::block::{BlockScope, BlockVariable};
use crate::ast;

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol(pub String);

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Symbol {
    pub fn new(s: &str) -> Self {
        Self(s.to_owned())
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum UnresolvedInstruction {
    Instruction(Instruction),
    Imm(Register, Symbol),
    AddImm(Register, Symbol),
    AddImmSigned(Register, Symbol),
    JumpOffset(Symbol),
    Label(Symbol),
}

impl fmt::Display for UnresolvedInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Instruction(i) => write!(f, "{i}"),
            Self::Imm(r, s) => write!(f, "Imm {r} !{s}"),
            Self::AddImm(r, s) => write!(f, "AddImm {r} !{s}"),
            Self::AddImmSigned(r, s) => write!(f, "AddImmSigned {r} !{s}"),
            Self::JumpOffset(s) => write!(f, "JumpOffset !{s}"),
            Self::Label(s) => write!(f, ":{s}"),
        }
    }
}

impl UnresolvedInstruction {
    pub fn resolve(&self, ctx: &Context) -> Result<Option<Instruction>, CompilerError> {
        match self {
            Self::Instruction(i) => Ok(Some(i.clone())),
            Self::Imm(reg, sym) => ctx.get(sym).and_then(|v| Literal12Bit::new_checked(v as u16).map_err(|_| CompilerError::LiteralOutOfBounds(v, 0, 0xfff)).map(|x| Some(Instruction::Imm(reg.clone(), x)))),
            Self::AddImm(reg, sym) => ctx.get(sym).and_then(|v| Literal7Bit::new_checked(v as u8).map_err(|_| CompilerError::LiteralOutOfBounds(v, 0, 0x7f)).map(|x| Some(Instruction::AddImm(reg.clone(), x)))),
            Self::AddImmSigned(reg, sym) => ctx.get(sym).and_then(|v| Literal7Bit::new_checked(v as u8).map_err(|_| CompilerError::LiteralOutOfBounds(v, 0, 0x7f)).map(|x| Some(Instruction::AddImmSigned(reg.clone(), x)))),
            Self::JumpOffset(sym) => ctx.get(sym).and_then(|v| Literal10Bit::new_checked(v as u16).map_err(|_| CompilerError::LiteralOutOfBounds(v, 0, 0x3ff)).map(|x| Some(Instruction::JumpOffset(x)))),
            Self::Label(_) => Ok(None),
        }
    }

    pub fn size(&self) -> u32 {
        match self {
            Self::Label(_) => 0,
            _ => 2,
        }
    }

}

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    Int,
    Char,
    Void,
    Pointer(Box<Type>),
    UncheckedInt,
}

impl Type {
    fn max(&self, other: &Self) -> Self {
        // bias LHS
        if self.size_bytes() >= other.size_bytes() {
            self.clone()
        } else {
            other.clone()
        }
    }

    pub fn is_pointer(&self) -> bool {
        if let Self::Pointer(_) = self {
            true
        } else {
            false
        }
    }

    pub fn size_bytes(&self) -> usize {
        match self {
            Self::Int => 2,
            Self::Char => 1,
            Self::Void => 0,
            // TODO: long pointer?
            Self::Pointer(_) => 2,
            Self::UncheckedInt => 2,
        }
    }

    pub fn can_assign_from(&self, other: &Self) -> bool {
        *other != Type::Void && self.size_bytes() >= other.size_bytes() 
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => write!(f, "int"),
            Self::Char => write!(f, "char"),
            Self::Void => write!(f, "void"),
            Self::Pointer(t) => write!(f, "*{t}"),
            Self::UncheckedInt => write!(f, "int"),
        }
    }
}

impl From<ast::Type> for Type {
    fn from(value: ast::Type) -> Self {
        match value {
            ast::Type::Int => Self::Int,
            ast::Type::Char => Self::Char,
            ast::Type::Void => Self::Void,
            ast::Type::Pointer(t) => Self::Pointer(Box::new((*t).into())),
        }
    }
}

// TODO: maybe this should return an error
pub fn type_of(ctx: &Context, scope: &BlockScope, expr: &ast::Expression) -> Type {
    match expr {
        ast::Expression::LiteralInt(_) => Type::Int, 
        ast::Expression::LiteralChar(_) => Type::Char, 
        ast::Expression::Variable(name) => {
            if let Some(bv) = scope.get(ctx, &name) {
                match bv {
                    BlockVariable::Local(_, t) => t,
                    BlockVariable::Arg(_, t) => t,
                    BlockVariable::Const(_) => Type::Int,
                    BlockVariable::Global(_, t) => t,
                }
            } else if let Some(_) = ctx.symbols.get(name) {
                Type::Int 
            } else {
                // undefined variables become void to maximize error info?
                // alternate: cast to unchecked ints which cast to anything
                Type::Void
            }
        }
        ast::Expression::AddressOf(name) => {
             if let Some(bv) = scope.get(ctx, &name.0) {
                Type::Pointer(Box::new(match bv {
                    BlockVariable::Local(_, t) => t,
                    BlockVariable::Arg(_, t) => t,
                    // TODO: wtf do we do here
                    BlockVariable::Const(_) => Type::Int,
                    BlockVariable::Global(_, t) => t,
                }))
            } else if let Some(_) = ctx.symbols.get(&name.0) {
                Type::Pointer(Box::new(Type::Int))
            } else {
                panic!("cannot take addr of {name}");
            }
        }
        ast::Expression::Deref(expr) => {
            let inner_type = type_of(ctx, scope, expr);
            if let Type::Pointer(t) = inner_type {
                *t.clone()
            } else {
                Type::Void
            }
        }
        ast::Expression::Bracketed(expr) =>  {
            type_of(ctx, scope, expr)
        }
        ast::Expression::FunctionCall(name, _) => {
            if let Some(def) = ctx.function_defs.get(&name.0) {
                def.return_type.clone() 
            } else {
                Type::Void
            }
        }
        ast::Expression::BinOp(a, b, op) => {
            let type_a = type_of(ctx, scope, a);
            let type_b = type_of(ctx, scope, b);
            match op {
                ast::BinOp::Add 
                    | ast::BinOp::Subtract 
                    | ast::BinOp::Multiply 
                    => type_a.max(&type_b),
                ast::BinOp::Mod => type_a,
                ast::BinOp::Equal
                    | ast::BinOp::NotEqual
                    | ast::BinOp::GreaterThan
                    | ast::BinOp::GreaterThanEqual
                    | ast::BinOp::LessThan
                    | ast::BinOp::LessThanEqual
                    => Type::Int,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_assign() {
        assert!(Type::Int.can_assign_from(&Type::Int));
    }
}
