use crate::iil::{self, block};

/// all F-IIL instructions
pub trait Op {}

pub use crate::generated::iil::flat::{OpExpr, OpVoid};

pub enum Expr {
    Op(OpExpr),
}

pub type BlockVoidLocal = OpVoid;
pub type BlockValuedLocal = OpExpr;
pub type BlockLocal = block::BlockLocal<BlockVoidLocal, BlockValuedLocal>;

pub mod instruction {
    pub use crate::generated::iil::flat::instruction::*;
}
