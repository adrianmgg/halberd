use std::{borrow::Cow, marker::PhantomData};

use chumsky::span::Spanned;
use num_bigint::BigInt;
use num_rational::BigRational;

use crate::types;

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
    // fn iteratively_adjust_sidecars<>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NoSidecars;
impl Sidecars for NoSidecars {
    type Expr = ();
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Expr<'a, S: Sidecars = NoSidecars> {
    pub data: ExprData<'a, S>,
    pub sidecar: S::Expr,
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
}

impl<'a> From<ExprData<'a, NoSidecars>> for Expr<'a, NoSidecars> {
    fn from(data: ExprData<'a, NoSidecars>) -> Self {
        Expr { data, sidecar: () }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExprData<'a, S: Sidecars = NoSidecars> {
    LiteralInt(Spanned<LiteralInt>),
    LiteralFloat(Spanned<LiteralFloat>),
    LiteralBool(Spanned<bool>),
    InfixOp(Box<Expr<'a, S>>, Spanned<InfixOp>, Box<Expr<'a, S>>),
    Var(Spanned<&'a str>),
    Declaration {
        name: Spanned<&'a str>,
        value: Box<Expr<'a, S>>,
    },
    Block(Spanned<Block<'a, S>>),
}

impl<'a, S: Sidecars> ExprData<'a, S> {
    pub(crate) fn map_sidecar() {}
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiteralInt {
    pub r#type: types::Integer,
    pub value: BigInt,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiteralFloat {
    pub r#type: types::Float,
    pub value: BigRational,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Block<'a, S: Sidecars = NoSidecars> {
    pub(crate) exprs: Vec<Expr<'a, S>>,
    pub(crate) last: Option<Box<Expr<'a, S>>>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum InfixOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    DotProduct,
    CrossProduct,
    MatrixMultiply,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Function<'a, S: Sidecars = NoSidecars> {
    pub(crate) name: Spanned<Cow<'a, str>>,
    pub(crate) return_type: Spanned<types::Type>,
    pub(crate) args: Vec<FunctionArg<'a>>,
    // jury's out on if this is a good idea but i'm gonna try it
    pub(crate) body: Expr<'a, S>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FunctionArg<'a> {
    pub(crate) name: Spanned<Cow<'a, str>>,
    pub(crate) r#type: Spanned<types::Type>,
}
