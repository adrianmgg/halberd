use crate::{
    iil::{self, block},
    util::impl_conversion_enum_variant,
};

pub trait FlattenableToBlock {
    fn flatten(self, ctx: &mut block::Ctx) -> crate::iil::h::Block;
}

pub use crate::generated::iil::hierarchical::{OpExpr, OpVoid};

// FIXME remove this if don't end up using it
pub enum OpOrBlock {
    Expr(OpExpr),
    Void(OpVoid),
    Block(Block),
}
impl_conversion_enum_variant!(OpOrBlock::Expr(OpExpr));
impl_conversion_enum_variant!(OpOrBlock::Void(OpVoid));
impl_conversion_enum_variant!(OpOrBlock::Block(Block));

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
