use std::borrow::Cow;

use chumsky::span::{SimpleSpan, Spanned};

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub(crate) struct FunctionDefinition<'a> {
//     pub name: Cow<'a, str>,
// }

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Expr<'a> {
    // FIXME temporary, need to fully implement literals
    Literal(u64),
    LiteralBool(bool),
    InfixOp(Box<Spanned<Self>>, InfixOp, Box<Spanned<Self>>),
    Var(&'a str),
    Declaration {
        name: Spanned<&'a str>,
        value: Box<Spanned<Self>>,
    },
    FunctionDeclaration {
        name: Spanned<&'a str>,
        body: Box<Spanned<Self>>,
    },
    Block {
        exprs: Vec<Spanned<Self>>,
        last: Option<Box<Spanned<Self>>>,
    },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Type {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Function<'a> {
    pub(crate) name: Cow<'a, str>,
    pub(crate) args: Vec<FunctionArg<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FunctionArg<'a> {
    pub(crate) name: Cow<'a, str>,
    pub(crate) r#type: Type,
}
