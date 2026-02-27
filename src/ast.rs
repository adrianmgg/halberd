use chumsky::span::Spanned;

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
