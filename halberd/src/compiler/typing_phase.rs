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
    types::{self, prelude::*},
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

    // infer types
    e.iteratively_modify_sidecars_2(&mut universe, (), &SidecarFns {
        func: |universe: &mut scope::Universe<_>,
               data: &ast::FunctionData<'a, PhasePartiallyTyped>,
               scope: &mut <PhasePartiallyTyped as Sidecars>::Func,
               _| {
            // dbg!(&universe);
            let mut scope_bound = universe.get_scope_mut(*scope);
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
                                item.r#type = Some(arg.r#type.inner.clone());
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
                let name = name.inner;
                let r#type = &r#type.inner;
                assert!(
                    universe
                        .get_scope_mut(scope)
                        .lookup_and_modify(name, |i: &mut NamespaceItemPartiallyTyped| i.r#type =
                            Some(r#type.clone()))
                );
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

pub(crate) fn infer_expr_type(
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
                    .and_then(|(lhs, rhs)| (lhs == rhs).then_some(lhs))
                    .cloned(),
                ast::InfixOp::Divide => todo!(),
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
            .and_then(|i| i.r#type.clone()),
        ast::ExprData::Declaration { name: _, r#type: _, value: _ } => Some(types::Void.into()),
        ast::ExprData::Block(spanned) => match &spanned.inner.last {
            // blocks with no terminal expression get the type void
            None => Some(types::Void.into()),
            // blocks with a terminal expression get that expression's type if it has one
            Some(terminal) => terminal.sidecar.r#type().clone(),
        },
    }
}
