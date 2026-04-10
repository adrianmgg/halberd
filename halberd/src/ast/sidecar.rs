use crate::ast::{Block, Expr, ExprData};
use chumsky::span::Spanned;

pub(crate) trait Sidecars {
    type Expr: std::fmt::Debug + Clone + PartialEq;
}

pub(crate) struct SidecarFns<ExprFn> {
    pub expr: ExprFn,
}

pub(crate) trait Sidecarred<'a, S: Sidecars> {
    type WithOtherSidecar<S2: Sidecars>;
    fn map_sidecars<S2: Sidecars, MapExpr: Fn(&ExprData<'a, S>, S::Expr) -> S2::Expr>(
        self,
        fns: &SidecarFns<MapExpr>,
    ) -> Self::WithOtherSidecar<S2>;
    // FIXME name
    fn modify_some_sidecars<AdjustExpr: Fn(&ExprData<'a, S>, &mut S::Expr) -> bool>(
        &mut self,
        fns: &SidecarFns<AdjustExpr>,
    ) -> usize;
    fn iteratively_modify_sidecars<AdjustExpr: Fn(&ExprData<'a, S>, &mut S::Expr) -> bool>(
        &mut self,
        fns: &SidecarFns<AdjustExpr>,
    ) {
        loop {
            if self.modify_some_sidecars(fns) == 0 {
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NoSidecars;
impl Sidecars for NoSidecars {
    type Expr = ();
}

impl<'a, S: Sidecars> Sidecarred<'a, S> for Expr<'a, S> {
    type WithOtherSidecar<S2: Sidecars> = Expr<'a, S2>;
    fn map_sidecars<
        S2: Sidecars,
        MapExpr: Fn(&ExprData<'a, S>, <S as Sidecars>::Expr) -> S2::Expr,
    >(
        self,
        fns: &SidecarFns<MapExpr>,
    ) -> Expr<'a, S2> {
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
        AdjustExpr: Fn(&ExprData<'a, S>, &mut <S as Sidecars>::Expr) -> bool,
    >(
        &mut self,
        fns: &SidecarFns<AdjustExpr>,
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
}
