use crate::compile::resolve::{Symbol, Type};
use crate::compile::codegen::RegisterStateError;

#[derive(Debug)]
pub enum CompilerError {
    LiteralOutOfBounds(u32, u32, u32),
    UnknownSymbol(Symbol),
    UnknownType(String),
    UnknownFunction(String),
    VariableAlreadyDefined(String),
    VariableUndefined(String),
    BreakNotInLoop,
    ContinueNotInLoop,
    InlineAsm(String),
    TypeAssign {
        from: Type,
        to: Type,
    },
    DerefInvalidType(Type),
    InvalidUntypedVariableDeclration(String),
    NonStructFieldReference(String, Type),
    StructFieldDoesNotExist(String, Type),
    ValueTooLargeForStack(Type),
    InvalidIndexType(Type),
    IncorrectFunctionArgCount {
        name: String,
        expected: usize,
        got: usize,
    },
    InstructionResolve(String),
    RegisterState(RegisterStateError),
}
