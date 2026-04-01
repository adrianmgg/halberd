use std::borrow::Cow;

use chumsky::span::Spanned;
use num_bigint::BigInt;
use num_rational::BigRational;

use crate::types;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Expr<'a> {
    LiteralInt(LiteralInt),
    LiteralFloat(LiteralFloat),
    LiteralBool(bool),
    InfixOp(Box<Spanned<Self>>, InfixOp, Box<Spanned<Self>>),
    Var(&'a str),
    Declaration {
        name: Spanned<&'a str>,
        value: Box<Spanned<Self>>,
    },
    Block(Block<'a>),
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
    pub(crate) exprs: Vec<Spanned<Expr<'a>>>,
    pub(crate) last: Option<Box<Spanned<Expr<'a>>>>,
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
    pub(crate) args: Vec<FunctionArg<'a>>,
    // jury's out on if this is a good idea but i'm gonna try it
    pub(crate) body: Spanned<Expr<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FunctionArg<'a> {
    pub(crate) name: Spanned<Cow<'a, str>>,
    pub(crate) r#type: Spanned<types::Type>,
}
