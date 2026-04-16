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

macro_rules! named_sidecars {
    ($($(#[$m:meta])* $name:ident = $expr_scope:ty, $expr_type:ty;)*) => {
        $($(#[$m])* pub(crate) type $name = sidecars::TheSidecars<ExprSidecar<$expr_scope, $expr_type>>; )*
    };
}
named_sidecars! {
    /// nothing
    NoSidecars = (), ();
    /// some scopes
    PhasePartiallyScoped = Option<ScopeId>, ();
    /// just scope
    PhaseFullyScoped = ScopeId, ();
    /// scope, and some types
    PhasePartiallyTyped = ScopeId, Option<types::Type>;
    /// scope and fully typed
    PhaseFullyTyped = ScopeId, types::Type;
}

pub fn foo<'a>(
    e: ast::Expr<'a, NoSidecars>,
) -> Result<ast::Expr<'a, PhaseFullyTyped>, Vec<ariadne::Report<'a>>> {
    let mut universe = scope::Universe::new();

    let mut e: ast::Expr<'a, PhasePartiallyScoped> =
        e.map_sidecars(&mut SidecarFns { expr: &mut |_, car| car.with_scope_none() });

    // populate scopes
    // FIXME this doesn't need to be using adding its own second level of Option...
    e.iteratively_modify_sidecars_2(&mut SidecarFns {
        expr: &mut |data,
                    car: &mut ExprSidecar<Option<ScopeId>, ()>,
                    ctx: SidecarWalkContexts<Option<ScopeId>>| {
            match car.scope_mut() {
                Some(scope) => (false, Some(*scope)),
                scope @ None => {
                    let super_scope = ctx.prior_sibling.flatten().or(ctx.parent);
                    if let Some(new_scope) =
                        super_scope.map(|s| universe.get_scope_mut(s).new_subscope())
                    {
                        *scope = Some(new_scope);
                        (true, Some(new_scope))
                    } else {
                        (false, None)
                    }
                }
            }
        },
    });

    // ensure all scopes populated
    e.validate_sidecars(&mut SidecarFns {
        expr: &mut |data, car| {
            car.scope().is_none().then_some(
                ariadne::Report::build(ariadne::ReportKind::Error, data.span().into_range())
                    .with_message("spans were not correctly applied everywhere! (this is an internal compiler error, not a problem with your code)")
                    .finish(),
            )
        },
    })?;
    let mut e: ast::Expr<'a, PhaseFullyScoped> = e.map_sidecars(&mut SidecarFns {
        expr: &mut |data, car| car.try_with_scope_definitely().unwrap(),
    });

    // fill in initial blanks for all types
    let mut e: ast::Expr<'a, PhasePartiallyTyped> = e.map_sidecars(&mut SidecarFns {
        expr: &mut |_, car: ExprSidecar<ScopeId, ()>| car.with_type_none(),
    });

    // now start actually populating the types
    e.iteratively_modify_sidecars(&mut SidecarFns {
        expr: |data: &ast::ExprData<'a, PhasePartiallyTyped>, sidecar: &mut ExprSidecar<_, _>| {
            match sidecar.type_mut() {
                Some(_) => false,
                sidecar_type @ None => {
                    let r#type = infer_expr_type(data);
                    match r#type {
                        None => false,
                        Some(r#type) => {
                            *sidecar_type = Some(r#type);
                            true
                        }
                    }
                }
            }
        },
    });

    e.validate_sidecars(&mut SidecarFns {
        expr: &mut |data, car| {
            car.r#type().is_none().then_some(
                ariadne::Report::build(ariadne::ReportKind::Error, data.span().into_range())
                    .with_message("unable to infer type")
                    .finish(),
            )
        },
    })?;
    let mut e: ast::Expr<'a, PhaseFullyTyped> = e.map_sidecars(&mut SidecarFns {
        expr: &mut |data, car| car.try_with_type_definitely().unwrap(),
    });

    Ok(e)
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
