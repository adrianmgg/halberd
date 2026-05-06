use super::{
    NamespaceItemFullyTyped, NamespaceItemPartiallyTyped, PhaseFullyScoped, PhaseFullyTyped,
    PhasePartiallyTyped,
};
use crate::{
    ast::{self, SidecarFns, Sidecarred as _, Sidecars},
    compiler::{
        ExprSidecar,
        sidecars::{ExprSidecarS as _, ExprSidecarT as _},
    },
    scope::{self, ScopeId},
    spv::operand_kind as ok,
    types::{self, prelude::*},
    util::matches_opt,
};

pub(crate) fn populate_types<'a>(
    e: ast::File<'a, PhaseFullyScoped>,
    universe: scope::Universe<<PhaseFullyScoped as Sidecars>::ScopeItem>,
) -> Result<
    (
        ast::File<'a, PhaseFullyTyped>,
        scope::Universe<NamespaceItemFullyTyped>,
    ),
    Vec<ariadne::Report<'a>>,
> {
    // fill in blanks for all the types
    let mut e: ast::File<'a, PhasePartiallyTyped> = e.map_sidecars(&mut SidecarFns {
        func: &mut |_, scope| scope,
        expr: &mut |_, car: ExprSidecar<ScopeId, ()>| car.with_type_none(),
    });
    let mut universe = universe.map(|()| NamespaceItemPartiallyTyped::default());

    // FIXME more temp hardcoded stuff. remove me eventually
    universe.root_scope_mut().lookup_and_modify("f_color", |a| {
        a.r#type = Some(
            types::Pointer {
                storage_class: ok::StorageClass::Output,
                target: Box::new(
                    types::Vector {
                        component_type: types::Float { width: 32 }.into(),
                        component_count: 4,
                    }
                    .into(),
                ),
            }
            .into(),
        );
    });
    universe.root_scope_mut().lookup_and_modify("uv", |a| {
        a.r#type = Some(
            types::Pointer {
                storage_class: ok::StorageClass::Input,
                target: Box::new(
                    types::Vector {
                        component_type: types::Float { width: 32 }.into(),
                        component_count: 2,
                    }
                    .into(),
                ),
            }
            .into(),
        );
    });
    universe.root_scope_mut().lookup_and_modify("time", |a| {
        a.r#type = Some(
            types::Pointer {
                storage_class: ok::StorageClass::Input,
                target: Box::new(types::Float { width: 32 }.into()),
            }
            .into(),
        );
    });

    // infer types
    e.iteratively_modify_sidecars_2(&mut universe, (), &SidecarFns {
        func: |universe: &mut scope::Universe<_>,
               data: &ast::FunctionData<'a, PhasePartiallyTyped>,
               scope: &mut <PhasePartiallyTyped as Sidecars>::Func,
               _| {
            dbg!(&universe);
            let mut scope_bound = universe.get_scope_mut(*scope);
            // FIXME wait hang on wait that doesn't quite sound right
            #[allow(
                clippy::map_all_any_identity,
                reason = "any() is short-circuiting, which we don't want here"
            )]
            let changed = (data.args.iter())
                .map(|arg| {
                    if matches!(
                        scope_bound.lookup(&arg.name),
                        Some(NamespaceItemPartiallyTyped { r#type: None })
                    ) {
                        // FIXME maybe making lookup_and_modify work like a `map` too would let us
                        //       write these better
                        let found = scope_bound.lookup_and_modify(
                            &arg.name,
                            |item: &mut NamespaceItemPartiallyTyped| {
                                item.r#type = Some(
                                    types::Pointer {
                                        storage_class: ok::StorageClass::Function,
                                        target: Box::new(arg.r#type.inner.clone()),
                                    }
                                    .into(),
                                );
                            },
                        );
                        assert!(found);
                        true
                    } else {
                        false
                    }
                })
                .any(std::convert::identity);
            (changed, ())
        },
        expr: |universe: &mut scope::Universe<_>,
               data: &_,
               sidecar: &mut <PhasePartiallyTyped as Sidecars>::Expr,
               _| {
            let scope = sidecar.scope();
            // FIXME need a check for assignment and declaration assignment to have types that
            //       line up
            if let ast::ExprData::Declaration { name, r#type, value } = data {
                let name = &name.inner;
                let r#type = &r#type.inner;
                assert!(universe.get_scope_mut(scope).lookup_and_modify(
                    name,
                    |i: &mut NamespaceItemPartiallyTyped| {
                        i.r#type = Some(
                            types::Pointer {
                                storage_class: ok::StorageClass::Function,
                                target: Box::new(r#type.clone()),
                            }
                            .into(),
                        );
                    }
                ));
            }
            match sidecar.type_mut() {
                Some(_) => (false, ()),
                sidecar_type @ None => {
                    let r#type = infer_expr_type(data, scope, universe);
                    match r#type {
                        None => (false, ()),
                        Some(r#type) => {
                            *sidecar_type = Some(r#type);
                            (true, ())
                        }
                    }
                }
            }
        },
    });
    e.validate_sidecars(&mut SidecarFns {
        func: &mut |_, _| None,
        expr: &mut |data, car| {
            car.r#type().is_none().then_some(
                ariadne::Report::build(ariadne::ReportKind::Error, data.span().into_range())
                    .with_code(3)
                    .with_message("unable to infer type")
                    .with_label(
                        ariadne::Label::new(data.span().into_range())
                            .with_message("at this expression"),
                    )
                    .finish(),
            )
        },
    })?;

    let mut e: ast::File<'a, PhaseFullyTyped> = e.map_sidecars(&mut SidecarFns {
        func: &mut |_, scope| scope,
        expr: &mut |data, car| car.try_with_type_definitely().unwrap(),
    });
    // FIXME don't just unwrap these
    let universe = universe.map(|item| NamespaceItemFullyTyped { r#type: item.r#type.unwrap() });
    Ok((e, universe))
}

fn infer_expr_type(
    data: &ast::ExprData<'_, PhasePartiallyTyped>,
    scope: ScopeId,
    universe: &mut scope::Universe<NamespaceItemPartiallyTyped>,
) -> Option<types::Type> {
    match data {
        ast::ExprData::LiteralInt(i) => Some(i.r#type.into()),
        ast::ExprData::LiteralFloat(f) => Some(f.r#type.into()),
        ast::ExprData::LiteralBool(_) => Some(types::Bool.into()),
        ast::ExprData::InfixOp(lhs, op, rhs) => {
            let lhs_type = lhs.sidecar.r#type();
            let rhs_type = rhs.sidecar.r#type();
            let lhs_and_rhs = try { (lhs_type.as_ref()?, rhs_type.as_ref()?) };
            match op.inner {
                ast::InfixOp::Add | ast::InfixOp::Subtract | ast::InfixOp::Multiply => lhs_and_rhs
                    .and_is_homogeneous()
                    .and_then(|ty| ty.and_is_vector_or_scalar_of().is_some().then_some(ty))
                    .cloned(),
                // FIXME need int div too
                ast::InfixOp::Divide => lhs_and_rhs.and_is_homogeneous().and_then(|ty| {
                    ty.and_is_number()
                        .and_is_float()
                        .map(Into::into)
                        .or_else(|| {
                            ty.and_is_vector().and_then(|v| {
                                v.component_type
                                    .and_is_float()
                                    .is_some()
                                    .then_some(v)
                                    .cloned()
                                    .map(Into::into)
                            })
                        })
                }),
                // OpDot
                ast::InfixOp::DotProduct => lhs_and_rhs
                    .and_is_homogeneous()
                    .and_is_vector()
                    .and_to_component_type()
                    .copied()
                    .map(Into::into),
                // glsl ext cross()
                ast::InfixOp::CrossProduct => lhs_and_rhs
                    .and_is_homogeneous()
                    .and_is_vector()
                    .and_has_n_components(3)
                    .copied()
                    .map(Into::into),
                // OpMatrixTimesMatrix
                ast::InfixOp::MatrixMultiply => {
                    try {
                        let lhs = lhs_type.and_is_matrix()?;
                        let rhs = rhs_type.and_is_matrix()?;
                        // "LeftMatrix must be a matrix whose Column Type is the same as the Column Type in Result Type."
                        // -> Result Type will be a matrix whose Column Type will be the Column Type from LeftMatrix
                        let column_type = lhs.column_type;
                        // "Result Type must be an OpTypeMatrix whose Column Type is a vector of floating-point type."
                        let component_type = column_type.component_type;
                        component_type.and_is_float()?;
                        // "RightMatrix must be a matrix with the same Component Type as the Component Type in Result Type."
                        (component_type == rhs.component_type()).then_some(())?;
                        // "[RightMatrix's] number of columns must equal the number of columns in Result Type."
                        let column_count = rhs.column_count();
                        // "[RightMatrix's] columns must have the same number of components as the number of columns in LeftMatrix."
                        (rhs.row_count() == lhs.column_count()).then_some(())?;
                        types::Matrix { column_type, column_count }.into()
                    }
                }
            }
        }
        ast::ExprData::Var(chumsky::span::Spanned { inner: name, .. }) => universe
            .get_scope(scope)
            .lookup(name)
            .and_then(|i| i.r#type.and_is_pointer().and_to_target().cloned()),
        // TODO should this be merged with the other handling of `Declaration`s elsewhere in this file?
        ast::ExprData::Declaration { name: _, r#type: _, value: _ } => Some(types::Void.into()),
        ast::ExprData::Block(spanned) => match &spanned.inner.last {
            // blocks with no terminal expression get the type void
            None => Some(types::Void.into()),
            // blocks with a terminal expression get that expression's type if it has one
            Some(terminal) => terminal.sidecar.r#type().clone(),
        },
        ast::ExprData::FunctionCall(fc) => infer_function_call_type(fc, scope, universe),
        ast::ExprData::FieldAccess(fa) => infer_field_access_type(fa, scope, universe),
        // FIXME needs check that assignment value is correct type
        ast::ExprData::Assignment { .. } => Some(types::Void.into()),
    }
}

fn infer_function_call_type(
    data: &ast::FunctionCall<'_, PhasePartiallyTyped>,
    scope: ScopeId,
    universe: &mut scope::Universe<NamespaceItemPartiallyTyped>,
) -> Option<types::Type> {
    // types of our args, or return None if any args not yet typed
    let arg_types: Vec<_> = data
        .args
        .iter()
        .map(|expr| expr.sidecar.r#type().as_ref())
        .try_collect()?;
    match &data.target {
        ast::IdentOrType::Ident(chumsky::span::Spanned { inner: name, span: _ }) => match name
            .as_ref()
        {
            "fract" | "exp" | "cos" | "sin" | "fabs" => matches_opt!(arg_types[..], [arg0] => arg0)
                .and_then(|ty| ty.and_is_vector_or_scalar_of().is_some().then_some(ty))
                .cloned(),
            "length" => matches_opt!(arg_types[..], [arg0] => arg0)
                .and_is_vector()
                .and_to_component_type()
                .and_is_float()
                .copied()
                .map(Into::into),
            "pow" => matches_opt!(arg_types[..], [a, b] => (a, b))
                .and_is_homogeneous()
                .and_then(|ty| ty.and_is_vector_or_scalar_of().is_some().then_some(ty))
                .cloned(),
            _ => None,
        },
        ast::IdentOrType::Type(chumsky::span::Spanned { inner: target, span }) => {
            if let Some(vec) = target.and_is_vector() {
                // > OpCompositeConstruct
                // > ... Result Type must be a composite type, whose top-level members/elements/components/columns have the same type as the types of the operands
                // > ... for constructing a vector, the operands may also be vectors with the same component type as the Result Type component type.
                // > If constructing a vector, the total number of components in all the operands must equal the number of components in Result Type.
                // -spec

                let expected_arg_type: types::Type = vec.component_type.into();

                arg_types
                    .into_iter()
                    // count up how many components are provided, short-circuiting out to a failure if
                    // any incorrectly-typed components provided
                    .map(|arg_type| {
                        (arg_type == &expected_arg_type).then_some(1).or_else(|| {
                            arg_type
                                .and_is_vector()
                                .and_has_component_type(&expected_arg_type)
                                .and_to_component_count()
                        })
                    })
                    .try_fold(0u32, |acc, cur| try { acc + cur? })
                    // require that to equal the expected number of components
                    .is_some_and(|component_count| component_count == vec.component_count)
                    // if so, we successfully construct one of those vectors
                    .then(|| target.clone())
            } else {
                None
            }
        }
    }
}

fn infer_field_access_type(
    data: &ast::FieldAccess<'_, PhasePartiallyTyped>,
    scope: ScopeId,
    universe: &mut scope::Universe<NamespaceItemPartiallyTyped>,
) -> Option<types::Type> {
    let Some(target_type) = data.target.sidecar.r#type() else {
        return None;
    };
    if let Some(vec) = target_type.and_is_vector() {
        let component_names = &(&['x', 'y', 'z', 'w'])[..vec.component_count as usize];
        if data.field.inner.chars().count() == 1
            && component_names.contains(&data.field.inner.chars().next().unwrap())
        {
            Some(vec.component_type.into())
        } else {
            None
        }
    } else if let Some(mat) = target_type.and_is_matrix() {
        todo!("handle typing for matrix field accesses")
    } else {
        None
    }
}
