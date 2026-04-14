use crate::ast::{Block, Expr, ExprData};
use chumsky::span::Spanned;

pub(crate) trait Sidecars {
    type Expr: std::fmt::Debug + Clone + PartialEq;
}

// FIXME rename this, we use it for more than just fns now
#[derive(Clone, Copy)]
pub(crate) struct SidecarFns<ExprFn> {
    pub expr: ExprFn,
}

// NOTE Default impl means our Default will set
//      `parent` to `Ctx::default()` and
//      `prior_sibling` to `None`, which is the behavior we want
#[derive(Debug, Default)]
pub(crate) struct SidecarWalkContexts<Ctx> {
    pub(crate) parent: Ctx,
    pub(crate) prior_sibling: Option<Ctx>,
}

pub(crate) trait Sidecarred<'a, S: Sidecars> {
    type WithOtherSidecar<S2: Sidecars>;
    fn map_sidecars<'f, S2: Sidecars, MapExpr: FnMut(&ExprData<'a, S>, S::Expr) -> S2::Expr>(
        self,
        fns: &mut SidecarFns<&mut MapExpr>,
    ) -> Self::WithOtherSidecar<S2>
    where
        'a: 'f;

    // FIXME name
    fn modify_some_sidecars<AdjustExpr: FnMut(&ExprData<'a, S>, &mut S::Expr) -> bool>(
        &mut self,
        fns: &mut SidecarFns<AdjustExpr>,
    ) -> usize;

    fn iteratively_modify_sidecars<AdjustExpr: FnMut(&ExprData<'a, S>, &mut S::Expr) -> bool>(
        &mut self,
        fns: &mut SidecarFns<AdjustExpr>,
    ) {
        loop {
            if self.modify_some_sidecars(fns) == 0 {
                break;
            }
        }
    }

    // NOTE trying out 'everything has the same ctx type' for now, since that solves the problem of
    //      how we api-wise e.g. specifically return an expr-ctx from an expr and so on,
    //      but if it causes other problems then maybe worth going back to the drawing board on
    //      that
    fn modify_some_sidecars_2<
        Ctx: Clone + Default,
        AdjustExpr: FnMut(&ExprData<'a, S>, &mut S::Expr, SidecarWalkContexts<Ctx>) -> (bool, Ctx),
    >(
        &mut self,
        fns: &mut SidecarFns<AdjustExpr>,
        ctxs: Option<SidecarWalkContexts<Ctx>>,
    ) -> (usize, Ctx);

    fn iteratively_modify_sidecars_2<
        Ctx: Clone + Default,
        AdjustExpr: FnMut(&ExprData<'a, S>, &mut S::Expr, SidecarWalkContexts<Ctx>) -> (bool, Ctx),
    >(
        &mut self,
        fns: &mut SidecarFns<AdjustExpr>,
    ) {
        loop {
            let (n, _) = self.modify_some_sidecars_2(fns, None);
            if n == 0 {
                break;
            }
        }
    }
}

impl<'a, S: Sidecars> Sidecarred<'a, S> for Expr<'a, S> {
    type WithOtherSidecar<S2: Sidecars> = Expr<'a, S2>;
    fn map_sidecars<
        'f,
        S2: Sidecars,
        MapExpr: FnMut(&ExprData<'a, S>, <S as Sidecars>::Expr) -> S2::Expr,
    >(
        self,
        fns: &mut SidecarFns<&mut MapExpr>,
    ) -> Expr<'a, S2>
    where
        'a: 'f,
    {
        Expr {
            sidecar: (fns.expr)(&self.data, self.sidecar),
            data: match self.data {
                ExprData::LiteralInt(i) => ExprData::LiteralInt(i),
                ExprData::LiteralFloat(f) => ExprData::LiteralFloat(f),
                ExprData::LiteralBool(b) => ExprData::LiteralBool(b),
                ExprData::InfixOp(lhs, op, rhs) => ExprData::InfixOp(
                    Box::new(lhs.map_sidecars(fns)),
                    op,
                    Box::new(rhs.map_sidecars(fns)),
                ),
                ExprData::Var(v) => ExprData::Var(v),
                ExprData::Declaration { name, value } => ExprData::Declaration {
                    name,
                    value: Box::new(value.map_sidecars(fns)),
                },
                ExprData::Block(Spanned {
                    inner: Block { exprs, last },
                    span,
                }) => ExprData::Block(Spanned {
                    span,
                    inner: Block {
                        exprs: exprs.into_iter().map(|e| e.map_sidecars(fns)).collect(),
                        last: last.map(|e| Box::new(e.map_sidecars(fns))),
                    },
                }),
            },
        }
    }

    fn modify_some_sidecars<
        AdjustExpr: FnMut(&ExprData<'a, S>, &mut <S as Sidecars>::Expr) -> bool,
    >(
        &mut self,
        fns: &mut SidecarFns<AdjustExpr>,
    ) -> usize {
        (if (fns.expr)(&self.data, &mut self.sidecar) {
            1
        } else {
            0
        }) + (match &mut self.data {
            ExprData::LiteralInt(_) => 0,
            ExprData::LiteralFloat(_) => 0,
            ExprData::LiteralBool(_) => 0,
            ExprData::InfixOp(lhs, _, rhs) => {
                lhs.modify_some_sidecars(fns) + rhs.modify_some_sidecars(fns)
            }
            ExprData::Var(_) => 0,
            ExprData::Declaration { name: _, value } => value.modify_some_sidecars(fns),
            ExprData::Block(b) => {
                (b.exprs)
                    .iter_mut()
                    .map(|e| e.modify_some_sidecars(fns))
                    .sum::<usize>()
                    + (b.last)
                        .as_mut()
                        .map(|e| e.modify_some_sidecars(fns))
                        .unwrap_or_default()
            }
        })
    }

    fn modify_some_sidecars_2<
        Ctx: Clone + Default,
        AdjustExpr: FnMut(&ExprData<'a, S>, &mut S::Expr, SidecarWalkContexts<Ctx>) -> (bool, Ctx),
    >(
        &mut self,
        fns: &mut SidecarFns<AdjustExpr>,
        ctxs: Option<SidecarWalkContexts<Ctx>>,
    ) -> (usize, Ctx) {
        let (changed, ctx_here) =
            (fns.expr)(&self.data, &mut self.sidecar, ctxs.unwrap_or_default());

        let mut n_changes = if changed { 1 } else { 0 };
        let mut ctx_final = ctx_here.clone();
        // ctx of most recently processed subexpression
        let mut ctx_subexpr = None;

        // https://youtu.be/NPwyyjtxlzU
        // TODO maybe refactor this to not use a macro lol
        macro_rules! foo {
            ($child_node:expr) => {
                #[allow(unused_assignments)]
                {
                    let (n, c) = $child_node.modify_some_sidecars_2(
                        fns,
                        Some(SidecarWalkContexts {
                            parent: ctx_here.clone(),
                            prior_sibling: ctx_subexpr.clone(),
                        }),
                    );
                    n_changes += n;
                    ctx_final = c.clone();
                    ctx_subexpr = Some(c);
                }
            };
        }

        match &mut self.data {
            ExprData::LiteralInt(_)
            | ExprData::LiteralFloat(_)
            | ExprData::LiteralBool(_)
            | ExprData::Var(_) => {}
            ExprData::InfixOp(lhs, _, rhs) => {
                foo!(lhs);
                foo!(rhs);
            }
            ExprData::Declaration { name: _, value } => {
                foo!(value);
            }
            ExprData::Block(b) => {
                for expr in b.exprs.iter_mut() {
                    foo!(expr);
                }
                if let Some(expr) = b.last.as_mut() {
                    foo!(expr);
                }
            }
        };

        (n_changes, ctx_final)
    }
}
