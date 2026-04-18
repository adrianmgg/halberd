use num_bigint::BigInt;
use num_rational::BigRational;

use crate::{
    iil::{self, block},
    types,
    util::impl_conversion_enum_variant,
};

pub trait FlattenableToBlock {
    fn flatten(self, ctx: &mut block::Ctx) -> crate::iil::h::Block;
}

pub use crate::generated::iil::hierarchical::{OpExpr, OpVoid};

pub enum Expr {
    Op(OpExpr),
    Constant(Constant),
}
impl_conversion_enum_variant!(Expr::{Op(OpExpr), Constant});

pub enum Constant {
    Int { r#type: types::Integer, value: BigInt },
    Float { r#type: types::Float, value: BigRational },
    Bool { value: bool },
}

impl Expr {
    pub fn flatten(self, ctx: &mut block::Ctx) -> BlockLocalExpr {
        match self {
            Expr::Op(op_expr) => BlockLocalExpr::Block(Box::new(op_expr.flatten(ctx))),
            Expr::Constant(constant) => todo!(),
        }
    }
}

pub type BlockLocalVoid = iil::flat::OpVoid;

pub enum BlockLocalExpr {
    Op(iil::flat::OpExpr),
    Block(Box<Block>),
    Constant(Constant),
}
impl_conversion_enum_variant!(BlockLocalExpr::{Op(iil::flat::OpExpr), Block(Box<Block>), Constant(Constant)});

pub type BlockTerminal = super::flat::OpExpr;
pub type Block = block::Block<BlockLocalVoid, BlockLocalExpr, Option<BlockTerminal>>;

pub mod instruction {
    pub use crate::generated::iil::hierarchical::instruction::*;
    use crate::{spv::operand_kind, types};

    pub struct OpFunction {
        pub control: enumset::EnumSet<operand_kind::FunctionControl>,
        pub r#type: types::Function,
        // pub body: Vec<super::OpOrBlock>,
        pub body: super::OpExpr,
    }
}
