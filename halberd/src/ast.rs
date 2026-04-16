mod sidecar;

use std::{borrow::Cow, collections::HashMap, fmt::Debug};

use chumsky::span::Spanned;
use derive_where::derive_where;
use num_bigint::BigInt;
use num_rational::BigRational;
pub(crate) use sidecar::*;

use crate::{compiler::NoSidecars, types};

#[derive_where(Debug, Clone, PartialEq; S::Expr, S::Func)]
pub(crate) struct Expr<'a, S: Sidecars = NoSidecars> {
    pub data: ExprData<'a, S>,
    pub sidecar: S::Expr,
}

impl<'a> From<ExprData<'a, NoSidecars>> for Expr<'a, NoSidecars> {
    fn from(data: ExprData<'a, NoSidecars>) -> Self { Expr { data, sidecar: Default::default() } }
}

#[derive_where(Debug, Clone, PartialEq; S::Expr, S::Func)]
pub(crate) enum ExprData<'a, S: Sidecars = NoSidecars> {
    LiteralInt(Spanned<LiteralInt>),
    LiteralFloat(Spanned<LiteralFloat>),
    LiteralBool(Spanned<bool>),
    InfixOp(Box<Expr<'a, S>>, Spanned<InfixOp>, Box<Expr<'a, S>>),
    Var(Spanned<&'a str>),
    Declaration { name: Spanned<&'a str>, value: Box<Expr<'a, S>> },
    Block(Spanned<Block<'a, S>>),
}

impl<'a, S: Sidecars> Expr<'a, S> {
    pub(crate) fn span(&self) -> chumsky::span::SimpleSpan { self.data.span() }
}

impl<'a, S: Sidecars> ExprData<'a, S> {
    pub(crate) fn span(&self) -> chumsky::span::SimpleSpan {
        match self {
            ExprData::LiteralInt(Spanned { span, .. })
            | ExprData::LiteralFloat(Spanned { span, .. })
            | ExprData::LiteralBool(Spanned { span, .. })
            | ExprData::Var(Spanned { span, .. })
            | ExprData::Block(Spanned { span, .. })
            | ExprData::InfixOp(_, Spanned { span, .. }, _)
            | ExprData::Declaration { name: Spanned { span, .. }, value: _ } => *span,
        }
    }
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

#[derive_where(Debug, Clone, PartialEq; S::Expr, S::Func)]
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

#[derive_where(Debug, Clone, PartialEq; S::Expr, S::Func)]
pub(crate) struct Function<'a, S: Sidecars = NoSidecars> {
    pub data: FunctionData<'a, S>,
    pub sidecar: S::Func,
}

impl<'a, S: Sidecars> Function<'a, S> {
    pub(crate) fn span(&self) -> chumsky::span::SimpleSpan { self.data.span() }
}

#[derive_where(Debug, Clone, PartialEq; S::Expr, S::Func)]
pub(crate) struct FunctionData<'a, S: Sidecars = NoSidecars> {
    pub(crate) name: Spanned<Cow<'a, str>>,
    pub(crate) return_type: Spanned<types::Type>,
    pub(crate) args: Vec<FunctionArg<'a>>,
    // jury's out on if this is a good idea but i'm gonna try it
    pub(crate) body: Expr<'a, S>,
}

impl<'a, S: Sidecars> FunctionData<'a, S> {
    pub(crate) fn span(&self) -> chumsky::span::SimpleSpan { self.name.span }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FunctionArg<'a> {
    pub(crate) name: Spanned<Cow<'a, str>>,
    pub(crate) r#type: Spanned<types::Type>,
}

#[derive_where(Debug; S::Expr, S::Func)]
#[derive_where(Default;)]
pub(crate) struct File<'a, S: Sidecars = NoSidecars> {
    pub(crate) functions: HashMap<Cow<'a, str>, Vec<Function<'a, S>>>,
}

impl<'a, S: Sidecars> chumsky::container::Container<Function<'a, S>> for File<'a, S> {
    fn push(&mut self, item: Function<'a, S>) {
        let a = item.data.name.inner.clone();
        self.functions.entry(a).or_default().push(item);
    }
}
