mod sidecar;

use std::{borrow::Cow, fmt::Debug};

use chumsky::span::Spanned;
use derive_where::derive_where;
use num_bigint::BigInt;
use num_rational::BigRational;
pub(crate) use sidecar::*;

use crate::{compiler::NoSidecars, types};

#[derive_where(Debug, Clone, PartialEq; S::Expr)]
pub(crate) struct Expr<'a, S: Sidecars = NoSidecars> {
    pub data: ExprData<'a, S>,
    pub sidecar: S::Expr,
}

impl<'a> From<ExprData<'a, NoSidecars>> for Expr<'a, NoSidecars> {
    fn from(data: ExprData<'a, NoSidecars>) -> Self {
        Expr {
            data,
            sidecar: Default::default(),
        }
    }
}

#[derive_where(Debug, Clone, PartialEq; S::Expr)]
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

#[derive_where(Debug, Clone, PartialEq; S::Expr)]
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

#[derive_where(Debug, Clone, PartialEq; S::Expr)]
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
