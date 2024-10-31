use crate::compile::codegen::expression::compile_expression;
use crate::compile::codegen::util::*;

use std::cell::RefCell;
use std::rc::Rc;

use simplevm::{
    resolve::UnresolvedInstruction, Instruction, Literal10Bit, Literal7Bit, Nibble, Register,
    StackOp, TestOp,
};

use crate::ast;
use crate::compile::block::{Block, BlockScope, BlockVariable, LoopLabels};
use crate::compile::context::Context;
use crate::compile::error::CompilerError;
use crate::compile::resolve::{type_of, Symbol, Type};
use crate::compile::util::*;

pub(super) fn compile_block(
    ctx: &mut Context,
    mut scope: BlockScope,
    statements: Vec<ast::Statement>,
) -> Result<Vec<UnresolvedInstruction>, CompilerError> {
    let mut out = Vec::new();
    for s in statements {
        match s {
            ast::Statement::Break => {
                if let Some(LoopLabels { ref bottom, .. }) = scope.loop_labels {
                    out.push(UnresolvedInstruction::Branch(bottom.to_string()));
                } else {
                    return Err(CompilerError::BreakNotInLoop);
                }
            }
            ast::Statement::Continue => {
                if let Some(LoopLabels { ref top, .. }) = scope.loop_labels {
                    out.push(UnresolvedInstruction::Branch(top.to_string()));
                } else {
                    return Err(CompilerError::ContinueNotInLoop);
                }
            }
            ast::Statement::While { cond, body } => {
                let block_identifier = gensym(rand::thread_rng());
                let label_test = Symbol::new(&(block_identifier.to_string() + "_while_lbl_test"));
                let label_out = Symbol::new(&(block_identifier + "_while_lbl_out"));
                out.push(UnresolvedInstruction::Label(label_test.to_string()));
                let mut compiled_cond = compile_expression(ctx, &mut scope, &cond)?;
                out.append(&mut compiled_cond);
                out.push(UnresolvedInstruction::Instruction(Instruction::Stack(
                    Register::C,
                    Register::SP,
                    StackOp::Pop,
                )));
                out.push(UnresolvedInstruction::Instruction(Instruction::Test(
                    Register::C,
                    Register::Zero,
                    TestOp::EitherNonZero,
                )));
                out.push(UnresolvedInstruction::Instruction(Instruction::BranchIf(
                    Literal10Bit::new_checked(4).unwrap(),
                )));
                out.push(UnresolvedInstruction::Branch(label_out.to_string()));
                let child_scope = scope.child_in_loop(label_test.clone(), label_out.clone());
                out.append(&mut compile_block(ctx, child_scope, body)?);
                out.push(UnresolvedInstruction::Branch(label_test.to_string()));
                out.push(UnresolvedInstruction::Label(label_out.to_string()));
            }
            ast::Statement::If {
                cond,
                body,
                else_body,
            } => {
                let block_identifier = gensym(rand::thread_rng());
                let label_true = Symbol::new(&(block_identifier.to_string() + "_if_lbl_true"));
                let label_out = Symbol::new(&(block_identifier + "_if_lbl_out"));
                let mut compiled_cond = compile_expression(ctx, &mut scope, &cond)?;
                out.append(&mut compiled_cond);
                // test if condition is FALSY
                out.push(UnresolvedInstruction::Instruction(Instruction::Stack(
                    Register::C,
                    Register::SP,
                    StackOp::Pop,
                )));
                out.push(UnresolvedInstruction::Instruction(Instruction::Test(
                    Register::C,
                    Register::Zero,
                    TestOp::BothZero,
                )));
                out.push(UnresolvedInstruction::Instruction(Instruction::BranchIf(
                    Literal10Bit::new_checked(4).unwrap(),
                )));
                out.push(UnresolvedInstruction::Branch(label_true.to_string()));
                // condition == FALSE
                if let Some(b) = else_body {
                    let child_scope = scope.child();
                    out.append(&mut compile_block(ctx, child_scope, b)?);
                };
                out.push(UnresolvedInstruction::Branch(label_out.to_string()));
                // condition == TRUE
                out.push(UnresolvedInstruction::Label(label_true.to_string()));
                let child_scope = scope.child();
                out.append(&mut compile_block(ctx, child_scope, body)?);
                out.push(UnresolvedInstruction::Branch(label_out.to_string()));
                out.push(UnresolvedInstruction::Label(label_out.to_string()));
            }
            ast::Statement::Declare(id, t, Some(expr)) => {
                if scope.get(ctx, &id.0).is_some() {
                    return Err(CompilerError::VariableAlreadyDefined(id.0.to_string()));
                }

                // type check
                let expr_type = type_of(ctx, &scope, &expr);
                let var_type = if let Some(tt) = t {
                    let var_type = Type::from_ast(ctx, &tt)?;
                    if var_type.is_struct() {
                        todo!("cannot declare struct value");
                    }
                    if !var_type.can_assign_from(&expr_type) {
                        return Err(CompilerError::TypeAssign {
                            from: expr_type,
                            to: var_type,
                        });
                    }
                    var_type
                } else {
                    expr_type
                };

                let local_offset = scope.define_local(&id.0, &var_type);
                // put expression on top of stack
                let mut compiled_expr = compile_expression(ctx, &mut scope, &expr)?;
                out.append(&mut compiled_expr);
                out.push(UnresolvedInstruction::Instruction(Instruction::Stack(
                    Register::C,
                    Register::SP,
                    StackOp::Pop,
                )));
                out.push(UnresolvedInstruction::Instruction(Instruction::Add(
                    Register::B,
                    Register::BP,
                    Register::Zero,
                )));
                out.push(UnresolvedInstruction::Instruction(Instruction::AddImm(
                    Register::B,
                    Literal7Bit::new_checked(local_offset as u8).unwrap(),
                )));
                write_value(&mut out, &var_type, Register::C, Register::B);
            }
            ast::Statement::Declare(id, t, None) => {
                if scope.get(ctx, &id.0).is_some() {
                    return Err(CompilerError::VariableAlreadyDefined(id.0.to_string()));
                }
                if let Some(tt) = t {
                    let declared_type = Type::from_ast(ctx, &tt)?;
                    scope.define_local(&id.0, &declared_type);
                } else {
                    return Err(CompilerError::InvalidUntypedVariableDeclration(
                        id.0.to_string(),
                    ));
                }
            }
            ast::Statement::Assign(id, expr) => {
                if let Some(bv) = scope.get(ctx, &id.0) {
                    match bv {
                        BlockVariable::Local(offset, ty) => {
                            let mut compiled_expr = compile_expression(ctx, &mut scope, &expr)?;
                            out.append(&mut compiled_expr);
                            assign_from_stack_to_local(&mut out, &ty, offset as u8);
                        }
                        BlockVariable::Arg(index, tt) => {
                            let expr_type = type_of(ctx, &scope, expr.as_ref());
                            if !tt.can_assign_from(&expr_type) {
                                return Err(CompilerError::TypeAssign {
                                    from: expr_type,
                                    to: tt,
                                });
                            }
                            let mut compiled_expr = compile_expression(ctx, &mut scope, &expr)?;
                            out.append(&mut compiled_expr);
                            assign_from_stack_to_arg(&mut out, index as u8);
                        }
                        BlockVariable::Global(addr, tt) => {
                            // type check
                            let expr_type = type_of(ctx, &scope, &expr);
                            if !tt.can_assign_from(&expr_type) {
                                return Err(CompilerError::TypeAssign {
                                    from: expr_type,
                                    to: tt,
                                });
                            }

                            let mut compiled_expr = compile_expression(ctx, &mut scope, &expr)?;
                            out.append(&mut compiled_expr);
                            out.push(UnresolvedInstruction::Instruction(Instruction::Stack(
                                Register::C,
                                Register::SP,
                                StackOp::Pop,
                            )));
                            if addr > 0xfff {
                                todo!("address too big: {addr}");
                            }
                            out.extend(load_address_to(addr, Register::B, Register::M));
                            write_value(&mut out, &tt, Register::C, Register::B);
                        }
                        _ => todo!("unimplemented {bv:?}"),
                    }
                } else {
                    return Err(CompilerError::VariableUndefined(id.0.to_string()));
                }
            }
            ast::Statement::AssignArray { lhs, index, rhs } => {
                let new_statement = ast::Statement::AssignDeref {
                    lhs: ast::Expression::BinOp(Box::new(lhs), Box::new(index), ast::BinOp::Add),
                    rhs,
                };
                out.extend(compile_block(ctx, scope.child(), vec![new_statement])?);
            }
            ast::Statement::AssignDeref { lhs, rhs } => {
                // TODO: check we can assign
                let lhs_type = type_of(ctx, &scope, &lhs);
                if let Type::Pointer(pointed_type) = lhs_type {
                    let compiled_addr = compile_expression(ctx, &mut scope, &lhs)?;
                    let compiled_value = compile_expression(ctx, &mut scope, &rhs)?;
                    out.extend(compiled_addr);
                    out.extend(compiled_value);
                    out.push(UnresolvedInstruction::Instruction(Instruction::Stack(
                        Register::B,
                        Register::SP,
                        StackOp::Pop,
                    )));
                    out.push(UnresolvedInstruction::Instruction(Instruction::Stack(
                        Register::C,
                        Register::SP,
                        StackOp::Pop,
                    )));
                    write_value(&mut out, &pointed_type, Register::B, Register::C);
                } else {
                    return Err(CompilerError::DerefInvalidType(lhs_type));
                }
            }
            ast::Statement::AssignStructField { fields, rhs } => {
                // println!("asssign struct field: {fields:?} = {rhs}");
                let compiled_expr = compile_expression(ctx, &mut scope, &rhs)?;
                out.extend(compiled_expr);
                if fields.is_empty() {
                    panic!("unreachable");
                }
                let head = fields.first().expect("parser issue");
                let head_var = scope
                    .get(ctx, &head.0)
                    .ok_or(CompilerError::VariableUndefined(head.0.to_string()))?;

                let var_type = match &head_var {
                    BlockVariable::Local(_, ty) => ty,
                    BlockVariable::Arg(_, ty) => ty,
                    BlockVariable::Global(_, ty) => ty,
                    BlockVariable::Const(_) => &Type::Int,
                };

                get_stack_field_offset(&mut out, &fields, var_type, &head_var, Register::C)?;

                // 2. pop value to write from stack
                out.push(UnresolvedInstruction::Instruction(Instruction::Stack(
                    Register::B,
                    Register::SP,
                    StackOp::Pop,
                )));
                // 3. write value
                write_value(
                    &mut out,
                    &type_of(ctx, &scope, &rhs),
                    Register::B,
                    Register::C,
                );
            }
            ast::Statement::Return(expr) => {
                let mut compiled_expr = compile_expression(ctx, &mut scope, &expr)?;
                out.append(&mut compiled_expr);
                // return in the A register
                out.push(UnresolvedInstruction::Instruction(Instruction::Stack(
                    Register::A,
                    Register::SP,
                    StackOp::Pop,
                )));
            }
            ast::Statement::Expression(expr) => {
                let mut compiled_expr = compile_expression(ctx, &mut scope, &expr)?;
                out.append(&mut compiled_expr);
                // forget what we just did
                out.push(UnresolvedInstruction::Instruction(Instruction::Stack(
                    Register::Zero,
                    Register::SP,
                    StackOp::Pop,
                )));
            }
        }
    }
    Ok(out)
}

pub(super) fn compile_body(
    ctx: &mut Context,
    statements: Vec<ast::Statement>,
    name: &str,
    args: Vec<(ast::Identifier, ast::Type)>,
) -> Result<Block, CompilerError> {
    let mut block = Block { ..Block::default() };
    block
        .instructions
        .push(UnresolvedInstruction::Label(name.to_string()));
    for (name, arg_type) in &args {
        block.define_arg(&name.0, &Type::from_ast(ctx, arg_type)?);
    }
    // function setup
    let local_count_sym = format!("__internal_{name}_local_count");
    block
        .instructions
        .push(UnresolvedInstruction::AddImm(Register::SP, local_count_sym));
    let cell = Rc::new(RefCell::new(block));
    let mut compiled = compile_block(ctx, BlockScope::new(cell.clone()), statements)?;
    {
        let mut block = cell.take();
        block.instructions.append(&mut compiled);
        // function exit
        // load return address -> C
        block.instructions.push(UnresolvedInstruction::Instruction(
            Instruction::LoadStackOffset(
                Register::C,
                Register::BP,
                Nibble::new_checked(1).unwrap(),
            ),
        ));
        // load previous SP = BP - 2
        block
            .instructions
            .push(UnresolvedInstruction::Instruction(Instruction::Add(
                Register::SP,
                Register::BP,
                Register::Zero,
            )));
        let offset = -4 - 2 * (args.len() as i8);
        block.instructions.push(UnresolvedInstruction::Instruction(
            Instruction::AddImmSigned(Register::SP, Literal7Bit::from_signed(offset).unwrap()),
        ));
        // load previous BP
        block.instructions.push(UnresolvedInstruction::Instruction(
            Instruction::LoadStackOffset(
                Register::BP,
                Register::BP,
                Nibble::new_checked(2).unwrap(),
            ),
        ));
        block
            .instructions
            .push(UnresolvedInstruction::Instruction(Instruction::AddImm(
                Register::C,
                Literal7Bit::new_checked(6).unwrap(),
            )));
        block.instructions.push(UnresolvedInstruction::Instruction(
            Instruction::JumpRegister(Register::Zero, Register::C),
        ));
        Ok(block)
    }
}
