use std::borrow::Cow;

use chumsky::span::Spanned;

use crate::types;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Expr<'a> {
    // FIXME temporary, need to fully implement literals
    LiteralNumber(LiteralNumber),
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
// TODO this is just a lazy temporary bodge representation of these
pub(crate) enum LiteralNumber {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    R32(f32),
    R64(f64),
}

impl LiteralNumber {
    pub fn r#type(&self) -> types::Type {
        use types::{Float, Integer};
        match self {
            Self::U8(_) => Integer::Unsigned(8).into(),
            Self::U16(_) => Integer::Unsigned(16).into(),
            Self::U32(_) => Integer::Unsigned(32).into(),
            Self::U64(_) => Integer::Unsigned(64).into(),
            Self::I8(_) => Integer::Signed(8).into(),
            Self::I16(_) => Integer::Signed(16).into(),
            Self::I32(_) => Integer::Signed(32).into(),
            Self::I64(_) => Integer::Signed(64).into(),
            Self::R32(_) => Float { width: 32 }.into(),
            Self::R64(_) => Float { width: 64 }.into(),
        }
    }
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
