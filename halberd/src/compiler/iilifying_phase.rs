use std::assert_matches;

use chumsky::span::Spanned;

use super::PhaseFullyTyped;
use crate::{
    ast::{self, Sidecars},
    compiler::{
        PhaseIILGeneration,
        sidecars::{ExprSidecarS, ExprSidecarT},
    },
    iil::{
        block::{self, Renumberable},
        f::{self, instruction as fops},
        h::{self, instruction as hops},
    },
    scope,
    spv::operand_kind as ok,
    types::{self, prelude::ExtAnyType},
    util::{Either, matches_opt},
};

pub(super) fn bar<'a>(
    file: ast::File<'a, PhaseIILGeneration>,
    universe: &mut scope::Universe<<PhaseIILGeneration as Sidecars>::ScopeItem>,
) {
    let mut blockctx = block::Ctx::new();
    for (name, functions) in file.functions.into_iter() {
        for function in functions.into_iter() {
            let f = foo(function, universe, &mut blockctx);
            println!("====================");
            dbg!(&f);
            println!(">>>>>>>>>>>>>>>>>>>>");
            let flat_body = flatten(f.body, &mut blockctx);
            dbg!(&flat_body);
        }
    }
}

fn foo<'a>(
    function: ast::Function<'a, PhaseIILGeneration>,
    universe: &mut scope::Universe<<PhaseIILGeneration as Sidecars>::ScopeItem>,
    blockctx: &mut block::Ctx,
) -> h::Function {
    h::Function {
        control: ok::FunctionControl::None,
        r#type: types::Function {
            args: function
                .data
                .args
                .iter()
                .map(|arg| arg.r#type.inner.clone())
                .collect(),
            result: Box::new(function.data.return_type.inner.clone()),
        },
        body: blockctx.new_block(|blockbuilder, blockctx| {
            let x = push_expr_to_block_mostly(function.data.body, universe, blockbuilder, blockctx);
            match x {
                block::BlockLocal::Void(void) => {
                    blockbuilder.push_void_local(void);
                    None
                }
                block::BlockLocal::Valued(valued) => Some(valued),
            }
        }),
    }
}

// TODO planning -
//      rather than giving the scope an Option<BlockLocalRef> for each namespace entry,
//      we could instead have some system for forward-declaring/reserving block entries before the
//      blocks have been created

/// inserts any intermediary things to the provided block,
/// and returns the (not yet inserted) block item representing this top-level expr
fn push_expr_to_block_mostly<'a>(
    expr: ast::Expr<'a, PhaseIILGeneration>,
    universe: &mut scope::Universe<<PhaseIILGeneration as Sidecars>::ScopeItem>,
    blockbuilder: &mut block::BlockBuilder<h::BlockLocalVoid, h::BlockLocalExpr>,
    blockctx: &mut block::Ctx,
) -> block::BlockLocal<h::BlockLocalVoid, h::BlockLocalExpr> {
    match expr.data {
        ast::ExprData::LiteralInt(Spanned { inner: ast::LiteralInt { r#type, value }, .. }) =>
            block::BlockLocal::Valued(h::Constant::Int { r#type, value }.into()),
        ast::ExprData::LiteralFloat(Spanned {
            inner: ast::LiteralFloat { r#type, value }, ..
        }) => block::BlockLocal::Valued(h::Constant::Float { r#type, value }.into()),
        ast::ExprData::LiteralBool(Spanned { inner: value, .. }) =>
            block::BlockLocal::Valued(h::Constant::Bool { value }.into()),
        ast::ExprData::InfixOp(lhs, op, rhs) => {
            let r#type = expr.sidecar.r#type();
            let lhs_blk = {
                let local = push_expr_to_block_mostly(*lhs, universe, blockbuilder, blockctx);
                let local = matches_opt!(local, block::BlockLocal::Valued(v) => v).unwrap();
                blockbuilder.push_valued_local(local)
            };
            let rhs_blk = {
                let local = push_expr_to_block_mostly(*rhs, universe, blockbuilder, blockctx);
                let local = matches_opt!(local, block::BlockLocal::Valued(v) => v).unwrap();
                blockbuilder.push_valued_local(local)
            };
            let x = match op.inner {
                ast::InfixOp::Add => match r#type.and_is_number().unwrap() {
                    types::NumberKind::Integer(_) => f::OpExpr::OpIAdd(fops::OpIAdd {
                        ret_type: r#type.clone(),
                        op0: lhs_blk,
                        op1: rhs_blk,
                    })
                    .into(),
                    types::NumberKind::Float(_) => f::OpExpr::OpFAdd(fops::OpFAdd {
                        ret_type: r#type.clone(),
                        op0: lhs_blk,
                        op1: rhs_blk,
                    })
                    .into(),
                },
                ast::InfixOp::Subtract => match r#type.and_is_number().unwrap() {
                    types::NumberKind::Integer(_) => f::OpExpr::OpISub(fops::OpISub {
                        ret_type: r#type.clone(),
                        op0: lhs_blk,
                        op1: rhs_blk,
                    })
                    .into(),
                    types::NumberKind::Float(_) => f::OpExpr::OpFSub(fops::OpFSub {
                        ret_type: r#type.clone(),
                        op0: lhs_blk,
                        op1: rhs_blk,
                    })
                    .into(),
                },
                ast::InfixOp::Multiply => match r#type.and_is_number().unwrap() {
                    types::NumberKind::Integer(_) => f::OpExpr::OpIMul(fops::OpIMul {
                        ret_type: r#type.clone(),
                        op0: lhs_blk,
                        op1: rhs_blk,
                    })
                    .into(),
                    types::NumberKind::Float(_) => f::OpExpr::OpFMul(fops::OpFMul {
                        ret_type: r#type.clone(),
                        op0: lhs_blk,
                        op1: rhs_blk,
                    })
                    .into(),
                },
                ast::InfixOp::Divide => todo!(),
                ast::InfixOp::DotProduct => todo!(),
                ast::InfixOp::CrossProduct => todo!(),
                ast::InfixOp::MatrixMultiply => todo!(),
            };
            block::BlockLocal::Valued(x)
        }
        ast::ExprData::Block(Spanned { inner: ast_block, .. }) => {
            let x: h::Block = blockctx.new_block(|b, ctx| {
                for ast_expr in ast_block.exprs.into_iter() {
                    push_expr_to_block_mostly(ast_expr, universe, b, ctx);
                }
                ast_block.last.and_then(|terminal| {
                    match push_expr_to_block_mostly(*terminal, universe, b, ctx) {
                        block::BlockLocal::Void(void) => {
                            blockbuilder.push_void_local(void);
                            None
                        }
                        block::BlockLocal::Valued(valued) => Some(valued),
                    }
                })
            });
            block::BlockLocal::Valued(h::BlockLocalExpr::Block(Box::new(x)))
        }
        ast::ExprData::Declaration { name, r#type, value } => {
            // add the OpVariable instruction declaring our var
            let var_br = blockbuilder.push_valued_local(
                f::OpExpr::OpVariable(fops::OpVariable {
                    ret_type: r#type.inner,
                    op0: ok::StorageClass::Function,
                    op1: None,
                })
                .into(),
            );
            // and add that to our corresponding scope entry
            let mut scope = universe.get_scope_mut(expr.sidecar.scope());
            assert!(scope.lookup_and_modify(name.inner, |info| {
                assert_matches!(info.block_ref, None);
                info.block_ref = Some(var_br);
            }));

            // add our initial value
            let value_br = push_expr_to_block_mostly(*value, universe, blockbuilder, blockctx);
            // FIXME don't unwrap
            let value_br = matches_opt!(value_br, block::BlockLocal::Valued(v) => v).unwrap();
            let value_br = blockbuilder.push_valued_local(value_br);

            // add an OpStore to save our initial value
            blockbuilder.push_void_local(f::OpVoid::OpStore(fops::OpStore {
                op0: var_br,
                op1: value_br,
                op2: None,
            }));

            // FIXME this should be able to be just None...
            block::BlockLocal::Void(f::OpVoid::OpNop(fops::OpNop))
        }
        ast::ExprData::Var(Spanned { inner: name, .. }) => {
            // FIXME need to do proper errors instead of panic here
            let scope = universe.get_scope(expr.sidecar.scope());
            let var_info = scope.lookup(name).unwrap();
            let block_ref = var_info.block_ref.unwrap();
            block::BlockLocal::Valued(block_ref.into())
        }
    }
}

pub(super) fn flatten(block: h::Block, blockctx: &mut block::Ctx) -> h::FlatBlock {
    blockctx.new_block(|blockbuilder, blockctx| flatten_into(block, blockbuilder))
}

fn flatten_into(
    block: h::Block,
    blockbuilder: &mut block::BlockBuilder<f::OpVoid, h::FlatBlockLocalExpr>,
) -> Option<h::FlatBlockLocalExpr> {
    let mut renumbers = Vec::<(block::BlockLocalRef, block::BlockLocalRef)>::new();
    let (locals, terminal) = block.into_parts();
    for (n, mut local) in locals {
        for (from, to) in renumbers.iter() {
            local.renumber(*from, *to);
        }
        match local {
            block::BlockLocal::Void(void) => {
                blockbuilder.push_void_local(void);
            }
            block::BlockLocal::Valued(valued) => {
                let x = match valued {
                    h::BlockLocalExpr::Op(o) => Either::Left(h::FlatBlockLocalExpr::Op(o)),
                    h::BlockLocalExpr::Constant(c) =>
                        Either::Left(h::FlatBlockLocalExpr::Constant(c)),
                    h::BlockLocalExpr::Ref(r) => Either::Left(h::FlatBlockLocalExpr::Ref(r)),
                    h::BlockLocalExpr::Block(b) => Either::Right(b),
                };
                match x {
                    Either::Left(l) => renumbers.push((n, blockbuilder.push_valued_local(l))),
                    Either::Right(b) =>
                        if let Some(terminal) = flatten_into(*b, blockbuilder) {
                            renumbers.push((n, blockbuilder.push_valued_local(terminal)));
                        },
                }
            }
        }
    }
    terminal.and_then(|e| match e {
        h::BlockLocalExpr::Op(o) => Some(h::FlatBlockLocalExpr::Op(o)),
        h::BlockLocalExpr::Constant(c) => Some(h::FlatBlockLocalExpr::Constant(c)),
        h::BlockLocalExpr::Ref(r) => Some(h::FlatBlockLocalExpr::Ref(r)),
        h::BlockLocalExpr::Block(b) => flatten_into(*b, blockbuilder),
    })
}
