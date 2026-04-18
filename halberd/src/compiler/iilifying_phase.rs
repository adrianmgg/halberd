use std::assert_matches;

use chumsky::span::Spanned;

use super::PhaseFullyTyped;
use crate::{
    ast::{self, Sidecars},
    compiler::{PhaseIILGeneration, sidecars::ExprSidecarS},
    iil::{
        self, block,
        f::{self, instruction as fops},
        h::{self, instruction as hops},
    },
    scope,
    spv::operand_kind as ok,
    types,
};

fn bar<'a>(
    file: ast::File<'a, PhaseFullyTyped>,
    universe: scope::Universe<<PhaseFullyTyped as Sidecars>::ScopeItem>,
) {
}

fn foo<'a>(
    function: ast::Function<'a, PhaseFullyTyped>,
    universe: &scope::Universe<<PhaseFullyTyped as Sidecars>::ScopeItem>,
) -> iil::h::instruction::OpFunction {
    iil::h::instruction::OpFunction {
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
        body: todo!(),
    }
}

// TODO planning -
//      rather than giving the scope an Option<BlockLocalRef> for each namespace entry,
//      we could instead have some system for forward-declaring/reserving block entries before the
//      blocks have been created

fn push_expr_to_block<'a>(
    expr: ast::Expr<'a, PhaseIILGeneration>,
    universe: &mut scope::Universe<<PhaseIILGeneration as Sidecars>::ScopeItem>,
    blockbuilder: &mut iil::block::BlockBuilder<iil::h::BlockLocalVoid, iil::h::BlockLocalExpr>,
    blockctx: &mut iil::block::Ctx,
) -> Option<block::BlockLocalRef> {
    match expr.data {
        ast::ExprData::LiteralInt(Spanned { inner: ast::LiteralInt { r#type, value }, .. }) =>
            Some(blockbuilder.push_valued_local(h::Constant::Int { r#type, value }.into())),
        ast::ExprData::LiteralFloat(Spanned {
            inner: ast::LiteralFloat { r#type, value }, ..
        }) => Some(blockbuilder.push_valued_local(h::Constant::Float { r#type, value }.into())),
        ast::ExprData::LiteralBool(Spanned { inner: value, .. }) =>
            Some(blockbuilder.push_valued_local(h::Constant::Bool { value }.into())),
        // TODO implement
        ast::ExprData::InfixOp(lhs, op, rhs) => todo!(),
        ast::ExprData::Block(Spanned { inner: ast_block, .. }) => Some(
            blockbuilder.push_valued_local(
                Box::new(blockctx.new_block(|b, ctx| {
                    for ast_expr in ast_block.exprs.into_iter() {
                        push_expr_to_block(ast_expr, universe, b, ctx);
                    }
                    ast_block
                        .last
                        .and_then(|terminal| push_expr_to_block(*terminal, universe, b, ctx))
                        .and_then(|terminal_ref| terminal_ref.into())
                }))
                .into(),
            ),
        ),
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
            let value_br = push_expr_to_block(*value, universe, blockbuilder, blockctx).unwrap();

            // add an OpStore
            blockbuilder.push_void_local(f::OpVoid::OpStore(fops::OpStore {
                op0: var_br,
                op1: value_br,
                op2: None,
            }));
            None
        }
        ast::ExprData::Var(Spanned { inner: name, .. }) => {
            // FIXME need to do proper errors instead of panic here
            let scope = universe.get_scope(expr.sidecar.scope());
            let var_info = scope.lookup(name).unwrap();
            let block_ref = var_info.block_ref.unwrap();
            block_ref.into()
        }
    }
}
