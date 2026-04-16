use crate::{
    ast::{self, SidecarFns, SidecarWalkContexts, Sidecarred, Sidecars},
    scope::{self, ScopeId},
    types::{self, prelude::*},
};

mod sidecars {
    use std::{
        fmt::{self, Debug},
        marker::PhantomData,
    };

    use crate::{scope::ScopeId, types::Type};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct ExprSidecarInner {
        scope: Option<ScopeId>,
        r#type: Option<Type>,
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct ExprSidecar<S, T>(ExprSidecarInner, PhantomData<(S, T)>);

    macro_rules! mk_exprsidecar_debug {
        ($s:ty, $t:ty, $self:ident $(,($field:literal, $val:expr))*) => {
            impl Debug for ExprSidecar<$s, $t> {
                fn fmt(&$self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.debug_struct("ExprSidecar")
                        $(.field($field, $val))*
                        .finish()
                }
            }
        };
    }
    mk_exprsidecar_debug!((), (), self);
    mk_exprsidecar_debug!(Option<ScopeId>, (), self, ("scope", &self.scope_maybe()));
    mk_exprsidecar_debug!(ScopeId, (), self, ("scope", &self.scope()));
    mk_exprsidecar_debug!(
        ScopeId,
        Option<Type>,
        self,
        ("scope", &self.scope()),
        ("type", &self.type_maybe())
    );
    mk_exprsidecar_debug!(
        ScopeId,
        Type,
        self,
        ("scope", &self.scope()),
        ("type", &self.r#type())
    );

    impl Default for ExprSidecar<(), ()> {
        fn default() -> Self { Self(ExprSidecarInner { scope: None, r#type: None }, PhantomData) }
    }

    impl<S, T> ExprSidecar<S, T> {
        pub fn with_scope_none(&self) -> ExprSidecar<Option<ScopeId>, T> {
            ExprSidecar(
                ExprSidecarInner { scope: None, r#type: self.0.r#type },
                PhantomData,
            )
        }

        // FIXME wait this should be taking self as owned shouldn't it uh oh
        pub fn with_scope(&self, id: ScopeId) -> ExprSidecar<ScopeId, T> {
            ExprSidecar(
                ExprSidecarInner { scope: Some(id), r#type: self.0.r#type },
                PhantomData,
            )
        }

        pub fn with_type_none(&self) -> ExprSidecar<S, Option<Type>> {
            ExprSidecar(
                ExprSidecarInner { scope: self.0.scope, r#type: None },
                PhantomData,
            )
        }

        pub fn with_type(&self, r#type: Type) -> ExprSidecar<S, Type> {
            ExprSidecar(
                ExprSidecarInner { scope: self.0.scope, r#type: Some(r#type) },
                PhantomData,
            )
        }
    }

    impl<T> ExprSidecar<Option<ScopeId>, T> {
        pub fn scope_maybe(&self) -> Option<ScopeId> { self.0.scope }

        pub fn scope_maybe_mut(&mut self) -> &mut Option<ScopeId> { &mut self.0.scope }

        pub fn try_with_scope_definitely(self) -> Option<ExprSidecar<ScopeId, T>> {
            if self.0.scope.is_none() {
                None
            } else {
                Some(ExprSidecar(self.0, PhantomData))
            }
        }
    }
    impl<T> ExprSidecar<ScopeId, T> {
        pub fn scope(&self) -> ScopeId { unsafe { self.0.scope.unwrap_unchecked() } }

        pub fn scope_mut(&mut self) -> &mut ScopeId {
            unsafe { self.0.scope.as_mut().unwrap_unchecked() }
        }
    }
    impl<S> ExprSidecar<S, Option<Type>> {
        pub fn type_maybe(&self) -> Option<Type> { self.0.r#type }

        pub fn type_maybe_mut(&mut self) -> &mut Option<Type> { &mut self.0.r#type }

        pub fn try_with_type_definitely(self) -> Option<ExprSidecar<S, Type>> {
            if self.0.r#type.is_none() {
                None
            } else {
                Some(ExprSidecar(self.0, PhantomData))
            }
        }
    }
    impl<S> ExprSidecar<S, Type> {
        pub fn r#type(&self) -> Type { unsafe { self.0.r#type.unwrap_unchecked() } }

        pub fn type_mut(&mut self) -> &mut Type {
            unsafe { self.0.r#type.as_mut().unwrap_unchecked() }
        }
    }

    impl<T> From<ExprSidecar<(), T>> for ExprSidecar<Option<ScopeId>, T> {
        fn from(value: ExprSidecar<(), T>) -> Self { Self(value.0, PhantomData) }
    }
    impl<T> TryFrom<ExprSidecar<Option<ScopeId>, T>> for ExprSidecar<ScopeId, T> {
        type Error = ();

        fn try_from(value: ExprSidecar<Option<ScopeId>, T>) -> Result<Self, Self::Error> {
            if value.scope_maybe().is_none() {
                Err(())
            } else {
                let inner = value.0;
                Ok(Self(inner, PhantomData))
            }
        }
    }

    impl<S> From<ExprSidecar<S, ()>> for ExprSidecar<S, Option<Type>> {
        fn from(value: ExprSidecar<S, ()>) -> Self { Self(value.0, PhantomData) }
    }
    impl<S> TryFrom<ExprSidecar<S, Option<Type>>> for ExprSidecar<S, Type> {
        type Error = ();

        fn try_from(value: ExprSidecar<S, Option<Type>>) -> Result<Self, Self::Error> {
            if value.type_maybe().is_none() {
                Err(())
            } else {
                let inner = value.0;
                Ok(Self(inner, PhantomData))
            }
        }
    }

    // FIXME give this a better name
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TheSidecars<Expr>(PhantomData<Expr>);
    impl<Expr: PartialEq + Clone + std::fmt::Debug> crate::ast::Sidecars for TheSidecars<Expr> {
        type Expr = Expr;
    }
}

pub(crate) use sidecars::ExprSidecar;

/// nothing
pub(crate) type NoSidecars = sidecars::TheSidecars<ExprSidecar<(), ()>>;
/// some scopes
pub(crate) type PhasePartiallyScoped = sidecars::TheSidecars<ExprSidecar<Option<ScopeId>, ()>>;
/// just scope
pub(crate) type PhaseFullyScoped = sidecars::TheSidecars<ExprSidecar<ScopeId, ()>>;
/// scope, and some types
pub(crate) type PhasePartiallyTyped =
    sidecars::TheSidecars<ExprSidecar<ScopeId, Option<types::Type>>>;
/// scope and fully typed
pub(crate) type PhaseFullyTyped = sidecars::TheSidecars<ExprSidecar<ScopeId, types::Type>>;

pub fn foo<'a>(
    e: ast::Expr<'a, NoSidecars>,
) -> Result<ast::Expr<'a, PhaseFullyTyped>, Vec<ariadne::Report<'a>>> {
    // we can trivially add a type already for anything whose type is definitive from just the
    // parsed ast, everything else we will need to do more work to figure out the type later

    let mut universe = scope::Universe::new();

    let mut e: ast::Expr<'a, PhasePartiallyScoped> =
        e.map_sidecars(&mut SidecarFns { expr: &mut |_, car| car.with_scope_none() });

    // populate scopes
    e.iteratively_modify_sidecars_2(&mut SidecarFns {
        expr: &mut |data,
                    car: &mut ExprSidecar<Option<ScopeId>, ()>,
                    ctx: SidecarWalkContexts<Option<ScopeId>>| {
            match car.scope_maybe_mut() {
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
            car.scope_maybe().is_none().then_some(
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
            match sidecar.type_maybe_mut() {
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
            car.type_maybe().is_none().then_some(
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
            let lhs_type = lhs.sidecar.type_maybe();
            let rhs_type = rhs.sidecar.type_maybe();
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
        ast::ExprData::Declaration { name: _, value } => value.sidecar.type_maybe(),
        ast::ExprData::Block(spanned) => match &spanned.inner.last {
            // blocks with no terminal expression get the type void
            None => Some(types::Type::Void),
            // blocks with a terminal expression get that expression's type if it has one
            Some(terminal) => terminal.sidecar.type_maybe(),
        },
    }
}
