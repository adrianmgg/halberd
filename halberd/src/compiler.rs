use crate::{
    ast::{self, SidecarFns, Sidecarred, Sidecars},
    scope::{self, ScopeId},
    types,
};

mod sidecars {
    use crate::{scope::ScopeId, types::Type};
    use std::marker::PhantomData;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct ExprSidecarInner {
        scope: Option<ScopeId>,
        r#type: Option<Type>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct ExprSidecar<S, T>(ExprSidecarInner, PhantomData<(S, T)>);

    impl Default for ExprSidecar<(), ()> {
        fn default() -> Self {
            Self(
                ExprSidecarInner {
                    scope: None,
                    r#type: None,
                },
                PhantomData,
            )
        }
    }

    impl<S, T> ExprSidecar<S, T> {
        pub fn with_scope(&self, id: ScopeId) -> ExprSidecar<ScopeId, T> {
            ExprSidecar(
                ExprSidecarInner {
                    scope: Some(id),
                    r#type: self.0.r#type,
                },
                PhantomData,
            )
        }
        pub fn with_type_none(&self) -> ExprSidecar<S, Option<Type>> {
            ExprSidecar(
                ExprSidecarInner {
                    scope: self.0.scope,
                    r#type: None,
                },
                PhantomData,
            )
        }
        pub fn with_type(&self, r#type: Type) -> ExprSidecar<S, Type> {
            ExprSidecar(
                ExprSidecarInner {
                    scope: self.0.scope,
                    r#type: Some(r#type),
                },
                PhantomData,
            )
        }
    }

    impl<T> ExprSidecar<Option<ScopeId>, T> {
        pub fn scope_maybe(&self) -> Option<ScopeId> {
            self.0.scope
        }
        pub fn scope_maybe_mut(&mut self) -> Option<&mut ScopeId> {
            self.0.scope.as_mut()
        }
    }
    impl<T> ExprSidecar<ScopeId, T> {
        pub fn scope(&self) -> ScopeId {
            unsafe { self.0.scope.unwrap_unchecked() }
        }
        pub fn scope_mut(&mut self) -> &mut ScopeId {
            unsafe { self.0.scope.as_mut().unwrap_unchecked() }
        }
    }
    impl<S> ExprSidecar<S, Option<Type>> {
        pub fn type_maybe(&self) -> Option<Type> {
            self.0.r#type
        }
        pub fn type_maybe_mut(&mut self) -> &mut Option<Type> {
            &mut self.0.r#type
        }
    }
    impl<S> ExprSidecar<S, Type> {
        pub fn r#type(&self) -> Type {
            unsafe { self.0.r#type.unwrap_unchecked() }
        }
        pub fn type_mut(&mut self) -> &mut Type {
            unsafe { self.0.r#type.as_mut().unwrap_unchecked() }
        }
    }

    impl<T> From<ExprSidecar<(), T>> for ExprSidecar<Option<ScopeId>, T> {
        fn from(value: ExprSidecar<(), T>) -> Self {
            Self(value.0, PhantomData)
        }
    }
    impl<T> TryFrom<ExprSidecar<Option<ScopeId>, T>> for ExprSidecar<ScopeId, T> {
        type Error = ();
        fn try_from(value: ExprSidecar<Option<ScopeId>, T>) -> Result<Self, Self::Error> {
            if value.scope().is_none() {
                Err(())
            } else {
                let inner = value.0;
                Ok(Self(inner, PhantomData))
            }
        }
    }

    impl<S> From<ExprSidecar<S, ()>> for ExprSidecar<S, Option<Type>> {
        fn from(value: ExprSidecar<S, ()>) -> Self {
            Self(value.0, PhantomData)
        }
    }
    impl<S> TryFrom<ExprSidecar<S, Option<Type>>> for ExprSidecar<S, Type> {
        type Error = ();
        fn try_from(value: ExprSidecar<S, Option<Type>>) -> Result<Self, Self::Error> {
            if value.r#type().is_none() {
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

    pub type NoSidecars = TheSidecars<ExprSidecar<(), ()>>;
}

pub(crate) use sidecars::{ExprSidecar, NoSidecars};

type Phase0 = NoSidecars;
type Phase1 = sidecars::TheSidecars<ExprSidecar<ScopeId, ()>>;
type Phase2 = sidecars::TheSidecars<ExprSidecar<ScopeId, Option<types::Type>>>;
type Phase3 = sidecars::TheSidecars<ExprSidecar<ScopeId, types::Type>>;

pub fn foo<'a>(e: ast::Expr<'a, NoSidecars>) {
    // we can trivially add a type already for anything whose type is definitive from just the
    // parsed ast, everything else we will need to do more work to figure out the type later

    let mut universe = scope::Universe::new();

    // populate scopes
    let mut e: ast::Expr<'a, Phase1> = e.map_sidecars(&mut SidecarFns {
        expr: |_: &ast::ExprData<'a, Phase0>, car: ExprSidecar<(), ()>| {
            // FIXME placeholder.
            car.with_scope(universe.root_scope_mut().new_subscope())
        },
    });

    // fill in initial blanks for all types
    let mut e: ast::Expr<'a, Phase2> = e.map_sidecars(&mut SidecarFns {
        expr: |_, car: ExprSidecar<ScopeId, ()>| car.with_type_none(),
    });

    // now start actually populating the types
    e.iteratively_modify_sidecars(&mut SidecarFns {
        expr: |data: &ast::ExprData<'a, Phase2>, sidecar: &mut ExprSidecar<_, _>| {
            match sidecar.type_maybe() {
                Some(_) => false,
                // FIXME rewrite this to handle the return bool automatically
                None => {
                    let r#type = match data {
                        ast::ExprData::LiteralInt(i) => Some(i.r#type.into()),
                        ast::ExprData::LiteralFloat(f) => Some(f.r#type.into()),
                        ast::ExprData::LiteralBool(_) => Some(types::Type::Bool),
                        ast::ExprData::InfixOp(lhs, op, rhs) => match op.inner {
                            ast::InfixOp::Add | ast::InfixOp::Subtract | ast::InfixOp::Multiply => {
                                todo!()
                            }
                            ast::InfixOp::Divide => todo!(),
                            ast::InfixOp::DotProduct => todo!(),
                            ast::InfixOp::CrossProduct => todo!(),
                            ast::InfixOp::MatrixMultiply => todo!(),
                        },
                        ast::ExprData::Var(_) => None,
                        // FIXME wait. is our ast wrong here WHOOPS
                        ast::ExprData::Declaration { name: _, value } => value.sidecar.type_maybe(),
                        ast::ExprData::Block(spanned) => match &spanned.inner.last {
                            // blocks with no terminal expression get the type void
                            None => Some(types::Type::Void),
                            // blocks with a terminal expression get that expression's type if it has one
                            Some(terminal) => terminal.sidecar.type_maybe(),
                        },
                    };
                    match r#type {
                        None => false,
                        Some(r#type) => {
                            sidecar.type_maybe_mut().insert(r#type);
                            true
                        }
                    }
                }
            }
        },
    });
}
