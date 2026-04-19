use num_bigint::BigInt;
use num_rational::BigRational;

use crate::{
    iil::{self, block},
    spv::operand_kind,
    types,
    util::impl_conversion_enum_variant,
};

pub trait FlattenableToBlock {
    fn flatten(self, ctx: &mut block::Ctx) -> crate::iil::h::Block;
}

pub use crate::generated::iil::hierarchical::{OpExpr, OpVoid};

#[derive(Debug)]
pub enum Expr {
    Op(OpExpr),
    Constant(Constant),
}
impl_conversion_enum_variant!(Expr::{Op(OpExpr), Constant});

#[derive(Debug)]
pub enum Constant {
    Int { r#type: types::Integer, value: BigInt },
    Float { r#type: types::Float, value: BigRational },
    Bool { value: bool },
}

#[derive(Debug)]
pub enum BlockLocalExpr {
    Op(iil::flat::OpExpr),
    Block(Box<Block>),
    Constant(Constant),
    Ref(block::BlockLocalRef),
}
impl_conversion_enum_variant!(BlockLocalExpr::{Op(iil::flat::OpExpr), Block(Box<Block>), Constant(Constant), Ref(block::BlockLocalRef)});

#[derive(Debug)]
pub enum FlatBlockLocalExpr {
    Op(iil::flat::OpExpr),
    Constant(Constant),
    Ref(block::BlockLocalRef),
}
impl_conversion_enum_variant!(FlatBlockLocalExpr::{Op(iil::flat::OpExpr), Constant(Constant), Ref(block::BlockLocalRef)});

pub type BlockLocalVoid = iil::flat::OpVoid;
pub type BlockTerminal = BlockLocalExpr;
pub type Block = block::Block<BlockLocalVoid, BlockLocalExpr, Option<BlockTerminal>>;
pub type FlatBlock = block::Block<BlockLocalVoid, FlatBlockLocalExpr, Option<BlockTerminal>>;

#[derive(Debug)]
pub struct Function {
    pub control: enumset::EnumSet<operand_kind::FunctionControl>,
    pub r#type: types::Function,
    pub body: Block,
}

pub mod instruction {
    pub use crate::generated::iil::hierarchical::instruction::*;
}
