use std::{
    assert_matches,
    borrow::Cow,
    collections::{HashMap, HashSet},
    convert::identity,
};

use chumsky::span::Spanned;

use super::PhaseFullyTyped;
use crate::{
    ast::{self, Sidecars},
    compiler::{
        PhaseIILGeneration, iil_phase_part2,
        sidecars::{ExprSidecarS, ExprSidecarT},
    },
    iil::{
        block::{self, Renumberable},
        f::{self, instruction as fops},
        flat::{IilOpExpr, IilOpExprUntyped, IilOpVoid},
        h,
    },
    scope,
    spv::{self, operand_kind as ok, writer::SpvWriter},
    types::{
        self,
        prelude::{ExtAnyType, ExtPointer},
    },
    util::{Either, Never, matches_opt},
};

pub(super) fn process_file(
    file: ast::File<'_, PhaseIILGeneration>,
    universe: &mut scope::Universe<<PhaseIILGeneration as Sidecars>::ScopeItem>,
    blockctx: &mut block::Ctx,
) {
    // insert each (name,function) sig in a block first, so we have block refs with which to refer
    // to all functions regardless of upcoming processing order.
    let forward_fns: block::Block<Never, _, ()> = blockctx.new_block(|blockbuilder, blockctx| {
        for (name, functions) in file.functions {
            for function in functions {
                // FIXME once we get support for calling functions (lol) need to insert this block
                //       ref in the namespace in the proper spot
                blockbuilder.push_valued_local((name.clone(), function));
            }
        }
    });

    // FIXME temp. hardcoded stuff, remove this all eventually
    let mut frag_color = None;
    let frag_color_type: types::Type = types::Pointer {
        storage_class: ok::StorageClass::Output,
        target: Box::new(
            types::Vector {
                component_type: types::Float { width: 32 }.into(),
                component_count: 4,
            }
            .into(),
        ),
    }
    .into();
    let mut main_inputs_block: block::Block<Never, f::OpExpr, ()> =
        blockctx.new_block(|blockbuilder, blockctx| {
            frag_color = Some(blockbuilder.push_valued_local(f::OpExpr::OpVariable(
                fops::OpVariable {
                    ret_type: frag_color_type.clone(),
                    op0: ok::StorageClass::Output,
                    op1: None,
                },
            )));
        });
    let frag_color = frag_color.unwrap();
    universe
        .root_scope_mut()
        .lookup_and_modify("f_color", |a| a.block_ref = Some(frag_color));

    let fns = forward_fns.map_mut(
        identity,
        |(_, function)| process_function(function, universe, blockctx),
        identity,
    );
    dbg!(&fns);

    let fns = fns.map_mut(
        identity,
        |func| h::FlatFunction {
            control: func.control,
            r#type: func.r#type,
            body: flatten(func.body, blockctx),
            is_main: func.is_main,
        },
        identity,
    );
    dbg!(&fns);

    eprintln!("================ type instructions ================");
    let mut all_types_needed: HashSet<types::Type> = fns
        .locals_valued_only()
        .flat_map(|(_, func)| func.types_referenced())
        .collect::<HashSet<_>>()
        .into_iter()
        .map(Cow::into_owned)
        .collect();
    all_types_needed.insert(frag_color_type);
    let (type_ops_block, mut type2local) =
        iil_phase_part2::types_to_asm(all_types_needed, blockctx);
    dbg!(&type_ops_block, &type2local);

    eprintln!("================ constant instructions ================");
    let all_constants_needed: HashSet<_> = fns
        .locals_valued_only()
        .flat_map(|(_, func)| func.constants_referenced())
        .collect();
    let (constant_ops_block, constant2local) =
        constants_to_asm(all_constants_needed.into_iter().cloned(), blockctx);
    dbg!(&constant_ops_block, &constant2local);

    eprintln!("================ sewn together ================");
    let sewn_together = sew_everything_together(
        blockctx,
        fns,
        type_ops_block,
        constant_ops_block,
        &constant2local,
        &mut type2local,
        main_inputs_block,
        frag_color,
    );
    dbg!(&sewn_together);

    // we can now directly map refs local to the sewn-together block into globally-qualified refs
    let map_local = |local: block::BlockLocalRef| -> u32 {
        sewn_together
            .relative_to(local)
            .unwrap()
            .try_into()
            .unwrap()
    };
    let map_refs = |local: block::BlockLocalRef| -> ok::IdRef { ok::IdRef(map_local(local)) };
    let map_types = |ty: types::Type| -> ok::IdResultType {
        ok::IdResultType(map_local(
            *type2local
                .get(&ty)
                .unwrap_or_else(|| panic!("no entry found for type {ty:?}")),
        ))
    };
    let mut writer = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("a.spv")
        .unwrap();
    let write_success = try {
        // magic number
        writer.write_word(spv::MAGIC)?;
        // version number
        writer.write_word(u32::from_be_bytes([
            0,
            spv::MAJOR_VERSION,
            spv::MINOR_VERSION,
            0,
        ]))?;
        // generator's magic number
        writer.write_word(spv::GENERATOR_MAGIC)?;
        // (reserved)
        writer.write_word(0)?;
        // ... instruction stream ...
        let max_id = sewn_together
            .locals_valued_only()
            .map(|(r, _)| map_local(r))
            .max()
            .unwrap_or_default();
        // bound
        writer.write_word(max_id + 1)?;
        for (r, op) in sewn_together.locals() {
            let result_id = ok::IdResult(map_local(r));
            // FIXME avoid unnecissary cloning in these
            match op.clone() {
                block::BlockLocal::Void(op) =>
                    op.into_spv_void(map_refs).write_instruction(&mut writer)?,
                block::BlockLocal::Valued(f::OpAnyValued::Typed(op)) => op
                    .into_spv_expr(map_refs, map_types)
                    .write_instruction(&mut writer, result_id)?,
                block::BlockLocal::Valued(f::OpAnyValued::Untyped(op)) => op
                    .into_spv_retuntyped(map_refs)
                    .write_instruction(&mut writer, result_id)?,
            }
        }
    };
}

#[allow(clippy::too_many_arguments, reason = "https://youtu.be/NPwyyjtxlzU")]
/// sew everything together. very hardcoded for now
fn sew_everything_together(
    blockctx: &mut block::Ctx,
    fns: block::Block<Never, h::FlatFunction, ()>,
    type_ops_block: block::Block<Never, f::OpExprUntyped, ()>,
    constant_ops_block: block::Block<Never, f::OpExpr, ()>,
    constant2local: &HashMap<h::Constant, block::BlockLocalRef>,
    type2local: &mut HashMap<types::Type, block::BlockLocalRef>,
    main_inputs_block: block::Block<Never, f::OpExpr, ()>,
    frag_color: block::BlockLocalRef,
) -> block::Block<f::OpVoid, f::OpAnyValued, ()> {
    let mut renumbers = HashMap::<block::BlockLocalRef, block::BlockLocalRef>::new();
    let mut sewn_together = blockctx.new_block(|blockbuilder, blockctx| {
        blockbuilder.push_void_local(f::OpVoid::OpCapability(fops::OpCapability {
            op0: ok::Capability::Shader,
        }));
        blockbuilder.push_valued_local(f::OpAnyValued::Untyped(f::OpExprUntyped::OpExtInstImport(
            fops::OpExtInstImport { op0: ok::LiteralString { value: "GLSL.std.450".into() } },
        )));
        blockbuilder.push_void_local(f::OpVoid::OpMemoryModel(fops::OpMemoryModel {
            op0: ok::AddressingModel::Logical,
            op1: ok::MemoryModel::GLSL450,
        }));

        let main_fn = fns
            .locals_valued_only()
            .find(|(_ref, function)| function.is_main)
            .map(|(r#ref, _function)| r#ref);

        if let Some(main_fn) = main_fn {
            blockbuilder.push_void_local(
                fops::OpEntryPoint {
                    op0: ok::ExecutionModel::Fragment,
                    op1: main_fn,
                    op2: ok::LiteralString { value: "main".into() },
                    op3: vec![frag_color],
                }
                .into(),
            );
            blockbuilder.push_void_local(f::OpVoid::OpDecorate(fops::OpDecorate {
                op0: frag_color,
                op1: ok::Decoration::Location(0u32.into()),
            }));
        }

        let (type_ops, ()) = type_ops_block.into_parts();
        for (r, t) in type_ops {
            renumbers.insert(
                r,
                blockbuilder.push_valued_local(t.into_valued_always().into()),
            );
        }

        let (constant_ops, ()) = constant_ops_block.into_parts();
        for (r, t) in constant_ops {
            renumbers.insert(
                r,
                blockbuilder.push_valued_local(t.into_valued_always().into()),
            );
        }

        let (main_input_ops, ()) = main_inputs_block.into_parts();
        for (r, t) in main_input_ops {
            renumbers.insert(
                r,
                blockbuilder.push_valued_local(t.into_valued_always().into()),
            );
        }

        let (fns, ()) = fns.into_parts();
        for (r, function) in fns {
            let function = function.into_valued_always();
            renumbers.insert(
                r,
                blockbuilder.push_valued_local(
                    f::OpExpr::OpFunction(fops::OpFunction {
                        ret_type: (*function.r#type.result).clone(),
                        control: ok::FunctionControl::None,
                        r#type: function.r#type.into(),
                    })
                    .into(),
                ),
            );

            let (body, terminal) = function.body.into_parts();
            for (r, o) in body {
                match o {
                    block::BlockLocal::Void(o) => blockbuilder.push_void_local(o),
                    block::BlockLocal::Valued(o) => {
                        renumbers.insert(r, match o {
                            h::FlatBlockLocalExpr::Op(o) =>
                                blockbuilder.push_valued_local(o.into()),
                            h::FlatBlockLocalExpr::OpUntyped(o) =>
                                blockbuilder.push_valued_local(o.into()),
                            h::FlatBlockLocalExpr::Constant(constant) => *constant2local
                                .get(&constant)
                                .unwrap_or_else(|| panic!("no constant found for {constant:?}")),
                            h::FlatBlockLocalExpr::Ref(block_local_ref) => block_local_ref,
                        });
                    }
                }
            }
            match terminal {
                None => blockbuilder.push_void_local(f::OpVoid::OpReturn(fops::OpReturn)),
                Some(ret) => {
                    let terminal_local = match ret {
                        h::FlatBlockLocalExpr::Op(o) => blockbuilder.push_valued_local(o.into()),
                        h::FlatBlockLocalExpr::OpUntyped(o) =>
                            blockbuilder.push_valued_local(o.into()),
                        h::FlatBlockLocalExpr::Constant(constant) => *constant2local
                            .get(&constant)
                            .unwrap_or_else(|| panic!("no constant found for {constant:?}")),
                        h::FlatBlockLocalExpr::Ref(block_local_ref) => block_local_ref,
                    };
                    blockbuilder.push_void_local(f::OpVoid::OpReturnValue(fops::OpReturnValue {
                        op0: terminal_local,
                    }));
                }
            }

            blockbuilder.push_void_local(f::OpVoid::OpFunctionEnd(fops::OpFunctionEnd));
        }
    });
    loop {
        let mut renumbered_any = false;
        for (from, to) in &renumbers {
            // renumber in the newly created sewn-together block
            renumbered_any |= sewn_together.renumber(*from, *to);
            // renumber in the existing type->local mapping
            for typelocal in type2local.values_mut() {
                renumbered_any |= typelocal.renumber(*from, *to);
            }
        }
        if !renumbered_any {
            break;
        }
    }
    sewn_together
}

fn process_function(
    function: ast::Function<'_, PhaseIILGeneration>,
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
                .map(|arg| {
                    types::Pointer {
                        storage_class: ok::StorageClass::Function,
                        target: Box::new(arg.r#type.inner.clone()),
                    }
                    .into()
                })
                .collect(),
            result: Box::new(function.data.return_type.inner.clone()),
        },
        body: blockctx.new_block(|blockbuilder, blockctx| {
            // add our args to the block
            for arg in &function.data.args {
                let r#ref = blockbuilder.push_valued_local(h::BlockLocalExpr::Op(
                    f::OpExpr::OpFunctionParameter(fops::OpFunctionParameter {
                        ret_type: types::Pointer {
                            storage_class: ok::StorageClass::Function,
                            target: Box::new(arg.r#type.inner.clone()),
                        }
                        .into(),
                    }),
                ));
                let mut scope = universe.get_scope_mut(function.sidecar);
                assert!(scope.lookup_and_modify(&arg.name, |info| {
                    assert_matches!(info.block_ref, None);
                    info.block_ref = Some(r#ref);
                }));
            }

            // start body
            blockbuilder.push_valued_local(h::BlockLocalExpr::OpUntyped(
                f::OpExprUntyped::OpLabel(fops::OpLabel),
            ));

            // add our body to the block
            let x = push_expr_to_block_mostly(function.data.body, universe, blockbuilder, blockctx);
            match x {
                block::BlockLocal::Void(void) => {
                    blockbuilder.push_void_local(void);
                    None
                }
                block::BlockLocal::Valued(valued) => Some(valued),
            }

            // (no OpFunctionEnd here b/c currently we wait until later when converting to spv to
            //  emit that)
        }),
        is_main: function.data.name.inner == "main",
    }
}

// TODO planning -
//      rather than giving the scope an Option<BlockLocalRef> for each namespace entry,
//      we could instead have some system for forward-declaring/reserving block entries before the
//      blocks have been created

/// inserts any intermediary things to the provided block,
/// and returns the (not yet inserted) block item representing this top-level expr
fn push_expr_to_block_mostly(
    expr: ast::Expr<'_, PhaseIILGeneration>,
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
                ast::InfixOp::Divide => todo!("IL for division"),
                ast::InfixOp::DotProduct => todo!("IL for dot product"),
                ast::InfixOp::CrossProduct => todo!("IL for cross product"),
                ast::InfixOp::MatrixMultiply => todo!("IL for matrix multiplication"),
            };
            block::BlockLocal::Valued(x)
        }
        ast::ExprData::Block(Spanned { inner: ast_block, .. }) => {
            let x: h::Block = blockctx.new_block(|b, ctx| {
                for ast_expr in ast_block.exprs {
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
            assert!(scope.lookup_and_modify(&name.inner, |info| {
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
            let var_info = scope.lookup(&name).unwrap();
            let var_type = var_info.r#type.and_is_pointer().and_to_target().unwrap();
            let block_ref = var_info.block_ref.unwrap();
            block::BlockLocal::Valued(
                f::OpExpr::OpLoad(fops::OpLoad {
                    ret_type: var_type.clone(),
                    op0: block_ref,
                    op1: None,
                })
                .into(),
            )
        }
        ast::ExprData::FunctionCall(function_call) => {
            let ast::FunctionCall { target, args, span: _ } = function_call;
            match target {
                ast::ExprOrType::Expr(expr) => todo!(),
                ast::ExprOrType::Type(target) =>
                    if let Some(vec) = target.and_is_vector() {
                        let args_pushed = args.into_iter().map(|arg| {
                            let arg =
                                push_expr_to_block_mostly(arg, universe, blockbuilder, blockctx);
                            let arg = matches_opt!(arg, block::BlockLocal::Valued(v) => v).unwrap();
                            blockbuilder.push_valued_local(arg)
                        });
                        block::BlockLocal::Valued(
                            f::OpExpr::OpCompositeConstruct(fops::OpCompositeConstruct {
                                ret_type: expr.sidecar.r#type().clone(),
                                op0: args_pushed.collect(),
                            })
                            .into(),
                        )
                    } else {
                        todo!()
                    },
            }
        }
        ast::ExprData::FieldAccess(ast::FieldAccess { target, field, span }) => {
            let target_type = target.sidecar.r#type();
            if let Some(vec) = target_type.and_is_vector() {
                let component_names = &(["x", "y", "z", "w"])[..vec.component_count as usize];
                let component_idx = component_names.iter().position(|name| *name == *field);
                if let Some(component_idx) = component_idx {
                    // FIXME using this same pattern up in InfixOp too. maybe factor out?
                    let target_local = {
                        let local =
                            push_expr_to_block_mostly(*target, universe, blockbuilder, blockctx);
                        let local = matches_opt!(local, block::BlockLocal::Valued(v) => v).unwrap();
                        blockbuilder.push_valued_local(local)
                    };
                    block::BlockLocal::Valued(
                        f::OpExpr::OpCompositeExtract(fops::OpCompositeExtract {
                            ret_type: expr.sidecar.r#type().clone(),
                            op0: target_local,
                            op1: vec![(component_idx as u32).into()],
                        })
                        .into(),
                    )
                } else {
                    panic!()
                }
            } else {
                panic!(
                    "trying to generate IL for field access which shouldnt have passed type-checking"
                );
            }
        }
        ast::ExprData::Assignment { target, value, span } => {
            // FIXME don't unwrap
            let var_br = universe
                .get_scope(expr.sidecar.scope())
                .lookup(target.inner.as_ref())
                .unwrap()
                .block_ref
                .unwrap();

            let value_br = push_expr_to_block_mostly(*value, universe, blockbuilder, blockctx);
            // FIXME don't unwrap
            let value_br = matches_opt!(value_br, block::BlockLocal::Valued(v) => v).unwrap();
            let value_br = blockbuilder.push_valued_local(value_br);

            block::BlockLocal::Void(f::OpVoid::OpStore(fops::OpStore {
                op0: var_br,
                op1: value_br,
                op2: None,
            }))
        }
    }
}

pub(super) fn flatten(block: h::Block, blockctx: &mut block::Ctx) -> h::FlatBlock {
    blockctx.new_block(|blockbuilder, blockctx| flatten_into(block, blockbuilder))
}

// FIXME probably some cases with intra-block forward refs which this doesn't handle properly...
fn flatten_into(
    block: h::Block,
    blockbuilder: &mut block::BlockBuilder<f::OpVoid, h::FlatBlockLocalExpr>,
) -> Option<h::FlatBlockLocalExpr> {
    let mut renumbers = Vec::<(block::BlockLocalRef, block::BlockLocalRef)>::new();
    let (locals, mut terminal) = block.into_parts();
    for (n, mut local) in locals {
        for (from, to) in &renumbers {
            local.renumber(*from, *to);
        }
        match local {
            block::BlockLocal::Void(void) => {
                blockbuilder.push_void_local(void);
            }
            block::BlockLocal::Valued(valued) => {
                let x = match valued {
                    h::BlockLocalExpr::Op(o) => Either::Left(h::FlatBlockLocalExpr::Op(o)),
                    h::BlockLocalExpr::OpUntyped(o) =>
                        Either::Left(h::FlatBlockLocalExpr::OpUntyped(o)),
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
    if let Some(ref mut terminal) = terminal {
        for (from, to) in &renumbers {
            terminal.renumber(*from, *to);
        }
    }
    terminal.and_then(|e| match e {
        h::BlockLocalExpr::Op(o) => Some(h::FlatBlockLocalExpr::Op(o)),
        h::BlockLocalExpr::OpUntyped(o) => Some(h::FlatBlockLocalExpr::OpUntyped(o)),
        h::BlockLocalExpr::Constant(c) => Some(h::FlatBlockLocalExpr::Constant(c)),
        h::BlockLocalExpr::Ref(r) => Some(h::FlatBlockLocalExpr::Ref(r)),
        h::BlockLocalExpr::Block(b) => flatten_into(*b, blockbuilder),
    })
}

pub(crate) fn constants_to_asm<Constants: Iterator<Item = h::Constant>>(
    constants_to_build: Constants,
    blockctx: &mut block::Ctx,
) -> (
    block::Block<Never, f::OpExpr, ()>,
    HashMap<h::Constant, block::BlockLocalRef>,
) {
    let mut constant2local = HashMap::new();
    let block = blockctx.new_block(|blockbuilder, blockctx| {
        for constant in constants_to_build {
            constant2local.insert(
                constant.clone(),
                blockbuilder.push_valued_local(match constant {
                    h::Constant::Int { r#type, value } => f::OpExpr::OpConstant(fops::OpConstant {
                        ret_type: r#type.into(),
                        op0: ok::LiteralInteger { value, r#type }.into(),
                    }),
                    h::Constant::Float { r#type, value } =>
                        f::OpExpr::OpConstant(fops::OpConstant {
                            ret_type: r#type.into(),
                            op0: ok::LiteralFloat { value, r#type }.into(),
                        }),
                    h::Constant::Bool { value: true } =>
                        f::OpExpr::OpConstantTrue(fops::OpConstantTrue {
                            ret_type: types::Bool.into(),
                        }),
                    h::Constant::Bool { value: false } =>
                        f::OpExpr::OpConstantFalse(fops::OpConstantFalse {
                            ret_type: types::Bool.into(),
                        }),
                }),
            );
        }
    });
    (block, constant2local)
}
