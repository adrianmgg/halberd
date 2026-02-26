#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Expr<'a> {
    Identifier(&'a str),
    // FIXME temporary, need to fully implement literals
    Literal(u64),
    LiteralBool(bool),
    InfixOp(Box<Self>, InfixOp, Box<Self>),
    Var(&'a str),
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
