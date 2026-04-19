use super::{PhaseFullyScoped, PhasePartiallyScoped};
use crate::{
    ast::{self, SidecarFns, SidecarWalkContexts, Sidecarred, Sidecars},
    compiler::{
        PhaseInitial,
        sidecars::{ExprSidecar, ExprSidecarS},
    },
    scope::{self, ScopeId},
};

pub(super) fn populate_scopes<'a>(
    e: ast::File<'a, PhaseInitial>,
    mut universe: scope::Universe<<PhaseInitial as Sidecars>::ScopeItem>,
) -> Result<
    (
        ast::File<'a, PhaseFullyScoped>,
        scope::Universe<<PhaseFullyScoped as Sidecars>::ScopeItem>,
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
        func: |universe: &mut scope::Universe<_>,
               data: &ast::FunctionData<_>,
               car: &mut _,
               _ctx| match car {
            Some(scope) => (false, *scope),
            scope @ None => {
                let new_scope = universe.root_scope_mut().new_subscope();
                // insert the function's args into its scope
                for arg in data.args.iter() {
                    universe
                        .get_scope_mut(new_scope)
                        .insert(arg.name.inner.clone(), ());
                }
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
                        ast::ExprData::Declaration { name, r#type: _, value: _ } => {
                            let new_scope = universe.get_scope_mut(super_scope).new_subscope();
                            universe.get_scope_mut(new_scope).insert(name.inner, ());
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
            .with_code(3)
            .with_message("spans were not correctly applied everywhere! (this is an internal compiler error, not a problem with your code)")
            .finish(),
    )
}
