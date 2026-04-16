use chumsky::span::Spanned;

use crate::ast::{Block, Expr, ExprData, FunctionData};

pub(crate) trait Sidecars {
    type Expr: std::fmt::Debug + Clone + PartialEq;
    type Func: std::fmt::Debug + Clone + PartialEq;
}

#[derive(Clone, Copy)]
pub(crate) struct SidecarFns<ExprFn, FuncFn> {
    pub expr: ExprFn,
    pub func: FuncFn,
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

    fn map_sidecars<
        'f,
        S2: Sidecars,
        MapExpr: FnMut(&ExprData<'a, S>, S::Expr) -> S2::Expr,
        MapFunc: FnMut(&FunctionData<'a, S>, S::Func) -> S2::Func,
    >(
        self,
        fns: &mut SidecarFns<&mut MapExpr, &mut MapFunc>,
    ) -> Self::WithOtherSidecar<S2>
    where
        'a: 'f;

    fn validate_sidecars_into<
        E,
        CheckExpr: FnMut(&ExprData<'a, S>, &S::Expr) -> Option<E>,
        CheckFunc: FnMut(&FunctionData<'a, S>, &S::Func) -> Option<E>,
    >(
        &self,
        fns: &mut SidecarFns<&mut CheckExpr, &mut CheckFunc>,
        errors: &mut Vec<E>,
    );

    fn validate_sidecars<
        E,
        CheckExpr: FnMut(&ExprData<'a, S>, &S::Expr) -> Option<E>,
        CheckFunc: FnMut(&FunctionData<'a, S>, &S::Func) -> Option<E>,
    >(
        &self,
        fns: &mut SidecarFns<&mut CheckExpr, &mut CheckFunc>,
    ) -> Result<(), Vec<E>> {
        let mut errs = Vec::new();
        self.validate_sidecars_into(fns, &mut errs);
        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }

    // FIXME name
    fn modify_some_sidecars<
        AdjustExpr: FnMut(&ExprData<'a, S>, &mut S::Expr) -> bool,
        AdjustFunc: FnMut(&FunctionData<'a, S>, &mut S::Func) -> bool,
    >(
        &mut self,
        fns: &mut SidecarFns<AdjustExpr, AdjustFunc>,
    ) -> usize;

    fn iteratively_modify_sidecars<
        AdjustExpr: FnMut(&ExprData<'a, S>, &mut S::Expr) -> bool,
        AdjustFunc: FnMut(&FunctionData<'a, S>, &mut S::Func) -> bool,
    >(
        &mut self,
        fns: &mut SidecarFns<AdjustExpr, AdjustFunc>,
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
        Global,
        Ctx: Clone + Default,
        AdjustExpr: FnMut(&mut Global, &ExprData<'a, S>, &mut S::Expr, SidecarWalkContexts<Ctx>) -> (bool, Ctx),
        AdjustFunc: FnMut(
            &mut Global,
            &FunctionData<'a, S>,
            &mut S::Func,
            SidecarWalkContexts<Ctx>,
        ) -> (bool, Ctx),
    >(
        &mut self,
        global: &mut Global,
        fns: &mut SidecarFns<&mut AdjustExpr, &mut AdjustFunc>,
        ctxs: Option<SidecarWalkContexts<Ctx>>,
    ) -> (usize, Ctx);

    fn iteratively_modify_sidecars_2<
        Global,
        Ctx: Clone + Default,
        AdjustExpr: FnMut(&mut Global, &ExprData<'a, S>, &mut S::Expr, SidecarWalkContexts<Ctx>) -> (bool, Ctx),
        AdjustFunc: FnMut(
            &mut Global,
            &FunctionData<'a, S>,
            &mut S::Func,
            SidecarWalkContexts<Ctx>,
        ) -> (bool, Ctx),
    >(
        &mut self,
        global: &mut Global,
        fns: &mut SidecarFns<&mut AdjustExpr, &mut AdjustFunc>,
    ) {
        loop {
            let (n, _) = self.modify_some_sidecars_2(global, fns, None);
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
        MapFunc: FnMut(&FunctionData<'a, S>, <S as Sidecars>::Func) -> S2::Func,
    >(
        self,
        fns: &mut SidecarFns<&mut MapExpr, &mut MapFunc>,
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
                ExprData::Declaration { name, value } =>
                    ExprData::Declaration { name, value: Box::new(value.map_sidecars(fns)) },
                ExprData::Block(Spanned { inner: Block { exprs, last }, span }) =>
                    ExprData::Block(Spanned {
                        span,
                        inner: Block {
                            exprs: exprs.into_iter().map(|e| e.map_sidecars(fns)).collect(),
                            last: last.map(|e| Box::new(e.map_sidecars(fns))),
                        },
                    }),
            },
        }
    }

    fn validate_sidecars_into<
        E,
        CheckExpr: FnMut(&ExprData<'a, S>, &<S as Sidecars>::Expr) -> Option<E>,
        CheckFunc: FnMut(&FunctionData<'a, S>, &<S as Sidecars>::Func) -> Option<E>,
    >(
        &self,
        fns: &mut SidecarFns<&mut CheckExpr, &mut CheckFunc>,
        errors: &mut Vec<E>,
    ) {
        if let Some(error) = (fns.expr)(&self.data, &self.sidecar) {
            errors.push(error);
        }
        match &self.data {
            ExprData::LiteralInt(_)
            | ExprData::LiteralFloat(_)
            | ExprData::LiteralBool(_)
            | ExprData::Var(_) => {}
            ExprData::InfixOp(lhs, _, rhs) => {
                lhs.validate_sidecars_into(fns, errors);
                rhs.validate_sidecars_into(fns, errors);
            }
            ExprData::Declaration { name: _, value } => {
                value.validate_sidecars_into(fns, errors);
            }
            ExprData::Block(Spanned { inner: block, .. }) => {
                for expr in block.exprs.iter() {
                    expr.validate_sidecars_into(fns, errors);
                }
                if let Some(last_expr) = &block.last {
                    last_expr.validate_sidecars_into(fns, errors);
                }
            }
        }
    }

    fn modify_some_sidecars<
        AdjustExpr: FnMut(&ExprData<'a, S>, &mut <S as Sidecars>::Expr) -> bool,
        AdjustFunc: FnMut(&FunctionData<'a, S>, &mut <S as Sidecars>::Func) -> bool,
    >(
        &mut self,
        fns: &mut SidecarFns<AdjustExpr, AdjustFunc>,
    ) -> usize {
        (if (fns.expr)(&self.data, &mut self.sidecar) {
            1
        } else {
            0
        }) + (match &mut self.data {
            ExprData::LiteralInt(_) => 0,
            ExprData::LiteralFloat(_) => 0,
            ExprData::LiteralBool(_) => 0,
            ExprData::InfixOp(lhs, _, rhs) =>
                lhs.modify_some_sidecars(fns) + rhs.modify_some_sidecars(fns),
            ExprData::Var(_) => 0,
            ExprData::Declaration { name: _, value } => value.modify_some_sidecars(fns),
            ExprData::Block(b) =>
                (b.exprs)
                    .iter_mut()
                    .map(|e| e.modify_some_sidecars(fns))
                    .sum::<usize>()
                    + (b.last)
                        .as_mut()
                        .map(|e| e.modify_some_sidecars(fns))
                        .unwrap_or_default(),
        })
    }

    fn modify_some_sidecars_2<
        Global,
        Ctx: Clone + Default,
        AdjustExpr: FnMut(&mut Global, &ExprData<'a, S>, &mut S::Expr, SidecarWalkContexts<Ctx>) -> (bool, Ctx),
        AdjustFunc: FnMut(
            &mut Global,
            &FunctionData<'a, S>,
            &mut S::Func,
            SidecarWalkContexts<Ctx>,
        ) -> (bool, Ctx),
    >(
        &mut self,
        global: &mut Global,
        fns: &mut SidecarFns<&mut AdjustExpr, &mut AdjustFunc>,
        ctxs: Option<SidecarWalkContexts<Ctx>>,
    ) -> (usize, Ctx) {
        let (changed, ctx_here) =
            (fns.expr)(global, &self.data, &mut self.sidecar, ctxs.unwrap_or_default());

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
                        global,
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
