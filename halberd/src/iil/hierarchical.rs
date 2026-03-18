use crate::iil::{self, block};

pub trait FlattenableToBlock {
    fn flatten(self, ctx: &mut block::Ctx) -> crate::iil::h::Block;
}

/// all H-IIL instructions
pub trait Op {
    fn flatten(self, ctx: &mut block::Ctx) -> Block;
}

pub use crate::generated::iil::hierarchical::{OpExpr, OpVoid};

pub enum Expr {
    Op(OpExpr),
    // ... constant, etc ...
}

impl Expr {
    pub fn flatten(self, ctx: &mut block::Ctx) -> BlockLocalExpr {
        match self {
            Expr::Op(op_expr) => BlockLocalExpr::Block(Box::new(op_expr.flatten(ctx))),
        }
    }
}

pub type BlockLocalVoid = OpVoid;
pub enum BlockLocalExpr {
    Op(iil::flat::OpExpr),
    Block(Box<Block>),
    // ... constant ...
}
pub type BlockTerminal = super::flat::OpExpr;
pub type Block = block::Block<BlockLocalVoid, BlockLocalExpr, BlockTerminal>;

pub mod instruction {
    pub use crate::generated::iil::hierarchical::instruction::*;
}
