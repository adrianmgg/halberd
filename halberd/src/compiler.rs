use crate::{
    ast::{self, SidecarFns, SidecarWalkContexts, Sidecarred, Sidecars},
    scope::{self, ScopeId},
    types::{self, prelude::*},
};

// FIXME need to disambiguate the naming between
//       sidecars (the AST's implementation of the sidecars system)
//       sidecars (the compiler's sidecar types for use with that system)
mod sidecars;

pub(crate) use sidecars::ExprSidecar;
use sidecars::{ExprSidecarS, ExprSidecarT};

/// nothing
pub(crate) struct NoSidecars;
impl Sidecars for NoSidecars {
    type Expr = ExprSidecar<(), ()>;
    type Func = ();
}
/// some scopes
pub(crate) struct PhasePartiallyScoped;
impl Sidecars for PhasePartiallyScoped {
    type Expr = ExprSidecar<Option<ScopeId>, ()>;
    type Func = Option<ScopeId>;
}
/// just scope
pub(crate) struct PhaseFullyScoped;
impl Sidecars for PhaseFullyScoped {
    type Expr = ExprSidecar<ScopeId, ()>;
    type Func = ScopeId;
}
/// scope, and some types
pub(crate) struct PhasePartiallyTyped;
impl Sidecars for PhasePartiallyTyped {
    type Expr = ExprSidecar<ScopeId, Option<types::Type>>;
    type Func = ScopeId;
}
/// scope and fully typed
pub(crate) struct PhaseFullyTyped;
impl Sidecars for PhaseFullyTyped {
    type Expr = ExprSidecar<ScopeId, types::Type>;
    type Func = ScopeId;
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NamespaceItemNothing;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NamespaceItemPartiallyTyped {
    pub(crate) r#type: Option<types::Type>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NamespaceItemFullyTyped {
    pub(crate) r#type: types::Type,
}

pub fn compile<'a>(
    e: ast::File<'a, NoSidecars>,
) -> Result<ast::File<'a, PhaseFullyTyped>, Vec<ariadne::Report<'a>>> {
    let mut universe = scope::Universe::new();

    let (e, universe) = populate_scopes(e, universe)?;
    let (e, universe) = populate_types(e, universe)?;

    Ok(e)
}

fn populate_types<'a>(
    e: ast::File<'a, PhaseFullyScoped>,
    universe: scope::Universe<NamespaceItemNothing>,
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
    let mut universe = universe.map(|_| NamespaceItemPartiallyTyped::default());

    // FIXME need to insert function argument types into universe too

    // infer types
    e.iteratively_modify_sidecars(&mut SidecarFns {
        func: |data: &_, scope: &mut _| false,
        expr: |data: &_, sidecar: &mut ExprSidecar<_, _>| match sidecar.type_mut() {
            Some(_) => false,
            sidecar_type @ None => {
                let r#type = infer_expr_type(data);
                match r#type {
                    None => false,
                    Some(r#type) => {
                        *sidecar_type = Some(r#type);
                        if let ast::ExprData::Declaration { name, value } = data {
                            assert!(
                                universe
                                    .get_scope_mut(sidecar.scope())
                                    .lookup_and_modify(name.inner, |x| x.r#type = Some(r#type))
                            );
                        }
                        true
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
                    .with_message("unable to infer type")
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

fn populate_scopes<'a>(
    e: ast::File<'a>,
    mut universe: scope::Universe<NamespaceItemNothing>,
) -> Result<
    (
        ast::File<'a, PhaseFullyScoped>,
        scope::Universe<NamespaceItemNothing>,
    ),
    Vec<ariadne::Report<'a>>,
> {
    // FIXME this should really just use a map instead of an iteratively modify,
    //       but i haven't written a map for the new API yet

    let mut e: ast::File<'a, PhasePartiallyScoped> = e.map_sidecars(&mut SidecarFns {
        func: &mut |_, _| None,
        expr: &mut |_, car| car.with_scope_none(),
    });

    let root_scope = universe.root_scope_id();
    e.iteratively_modify_sidecars_2(&mut universe, root_scope, &SidecarFns {
        func: |universe: &mut scope::Universe<_>, data: &_, car: &mut _, _ctx| match car {
            Some(scope) => (false, *scope),
            scope @ None => {
                let new_scope = universe.root_scope_mut().new_subscope();
                *scope = Some(new_scope);
                (true, new_scope)
            }
        },
        expr: |universe: &mut scope::Universe<_>,
               data: &_,
               car: &mut ExprSidecar<_, _>,
               ctx: SidecarWalkContexts<_>| {
            match car.scope_mut() {
                Some(scope) => (false, *scope),
                scope @ None => {
                    let super_scope = ctx.prior_sibling.unwrap_or(ctx.parent);
                    // TODO can we continue doing it this way (child nodes have *same* namespace
                    //      as their parent), or do we instead need to distinguish a sub-node from
                    //      the top one?
                    let new_scope = match data {
                        ast::ExprData::LiteralInt(..)
                        | ast::ExprData::LiteralFloat(..)
                        | ast::ExprData::LiteralBool(..)
                        | ast::ExprData::Var(..)
                        | ast::ExprData::InfixOp(..) => super_scope,
                        // blocks get a new scope
                        ast::ExprData::Block(..) =>
                            universe.get_scope_mut(super_scope).new_subscope(),
                        ast::ExprData::Declaration { name, value } => {
                            let new_scope = universe.get_scope_mut(super_scope).new_subscope();
                            universe
                                .get_scope_mut(new_scope)
                                .insert(name.inner, Default::default());
                            new_scope
                        }
                    };
                    *scope = Some(new_scope);
                    (true, new_scope)
                }
            }
        },
    });

    e.validate_sidecars(&mut SidecarFns {
        func: &mut |data, scope| validate_has_scope(*scope, data.span()),
        expr: &mut |data, car| validate_has_scope(car.scope(), data.span()),
    })?;
    let e: ast::File<'a, PhaseFullyScoped> = e.map_sidecars(&mut SidecarFns {
        func: &mut |data, scope| scope.unwrap(),
        expr: &mut |data, car| car.try_with_scope_definitely().unwrap(),
    });

    Ok((e, universe))
}

fn validate_has_scope<'a>(
    scope: Option<ScopeId>,
    span: chumsky::span::SimpleSpan,
) -> Option<ariadne::Report<'a>> {
    scope.is_none().then_some(
        ariadne::Report::build(ariadne::ReportKind::Error, span.into_range())
            .with_message("spans were not correctly applied everywhere! (this is an internal compiler error, not a problem with your code)")
            .finish(),
    )
}

fn infer_expr_type<'a>(data: &ast::ExprData<'a, PhasePartiallyTyped>) -> Option<types::Type> {
    match data {
        ast::ExprData::LiteralInt(i) => Some(i.r#type.into()),
        ast::ExprData::LiteralFloat(f) => Some(f.r#type.into()),
        ast::ExprData::LiteralBool(_) => Some(types::Type::Bool),
        ast::ExprData::InfixOp(lhs, op, rhs) => {
            let lhs_type = lhs.sidecar.r#type();
            let rhs_type = rhs.sidecar.r#type();
            let lhs_and_rhs = try { (lhs_type?, rhs_type?) };
            match op.inner {
                ast::InfixOp::Add | ast::InfixOp::Subtract | ast::InfixOp::Multiply =>
                    lhs_and_rhs.and_then(|(lhs, rhs)| (lhs == rhs).then_some(lhs)),
                ast::InfixOp::Divide => todo!(),
                // OpDot
                ast::InfixOp::DotProduct => lhs_and_rhs
                    .and_is_homogeneous()
                    .and_is_vector()
                    .and_to_component_type()
                    .and_is_float()
                    .map(Into::into),
                // glsl ext cross()
                ast::InfixOp::CrossProduct => lhs_and_rhs
                    .and_is_homogeneous()
                    .and_is_vector()
                    .and_has_n_components(3)
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
        ast::ExprData::Var(_) => None,
        // FIXME wait. is our ast wrong here WHOOPS
        ast::ExprData::Declaration { name: _, value } => value.sidecar.r#type(),
        ast::ExprData::Block(spanned) => match &spanned.inner.last {
            // blocks with no terminal expression get the type void
            None => Some(types::Type::Void),
            // blocks with a terminal expression get that expression's type if it has one
            Some(terminal) => terminal.sidecar.r#type(),
        },
    }
}
