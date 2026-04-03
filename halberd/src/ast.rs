use std::borrow::Cow;

use chumsky::span::Spanned;
use num_bigint::BigInt;
use num_rational::BigRational;

use crate::types;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Expr<'a> {
    LiteralInt(Spanned<LiteralInt>),
    LiteralFloat(Spanned<LiteralFloat>),
    LiteralBool(Spanned<bool>),
    InfixOp(Box<Self>, Spanned<InfixOp>, Box<Self>),
    Var(Spanned<&'a str>),
    Declaration {
        name: Spanned<&'a str>,
        value: Box<Self>,
    },
    Block(Spanned<Block<'a>>),
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
pub(crate) struct Block<'a> {
    pub(crate) exprs: Vec<Expr<'a>>,
    pub(crate) last: Option<Box<Expr<'a>>>,
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
pub(crate) struct Function<'a> {
    pub(crate) name: Spanned<Cow<'a, str>>,
    pub(crate) return_type: Spanned<types::Type>,
    pub(crate) args: Vec<FunctionArg<'a>>,
    // jury's out on if this is a good idea but i'm gonna try it
    pub(crate) body: Expr<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FunctionArg<'a> {
    pub(crate) name: Spanned<Cow<'a, str>>,
    pub(crate) r#type: Spanned<types::Type>,
}
