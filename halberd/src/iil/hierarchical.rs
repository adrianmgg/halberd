use num_bigint::BigInt;
use num_rational::BigRational;

use crate::{
    iil::{
        self,
        block::{self, Renumberable},
    },
    spv::operand_kind,
    types,
    util::impl_conversion_enum_variant,
};

pub trait FlattenableToBlock {
    fn flatten(self, ctx: &mut block::Ctx) -> crate::iil::h::Block;
}

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

impl Renumberable for BlockLocalExpr {
    fn renumber(&mut self, from: block::BlockLocalRef, to: block::BlockLocalRef) {
        match self {
            BlockLocalExpr::Op(op_expr) => op_expr.renumber(from, to),
            BlockLocalExpr::Block(block) => block.renumber(from, to),
            BlockLocalExpr::Constant(constant) => {}
            BlockLocalExpr::Ref(r) => r.renumber(from, to),
        }
    }
}

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
pub type FlatBlock = block::Block<BlockLocalVoid, FlatBlockLocalExpr, Option<FlatBlockLocalExpr>>;

#[derive(Debug)]
pub struct Function {
    pub control: enumset::EnumSet<operand_kind::FunctionControl>,
    pub r#type: types::Function,
    pub body: Block,
}

#[derive(Debug)]
pub struct FlatFunction {
    pub control: enumset::EnumSet<operand_kind::FunctionControl>,
    pub r#type: types::Function,
    pub body: FlatBlock,
}
