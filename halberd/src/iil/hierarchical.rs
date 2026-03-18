use enum_dispatch::enum_dispatch;

use crate::iil::{self, block};

#[enum_dispatch(OpExpr)]
trait FlattenableToBlock {
    fn flatten(self, ctx: &mut block::Ctx) -> Block;
}

/// all H-IIL instructions
pub trait Op {
    fn flatten(self, ctx: &mut block::Ctx) -> Block;
}

#[enum_dispatch]
/// a H-IIL instruction with a return value
pub enum OpExpr {
    OpIAdd(instruction::OpIAdd),
}

/// a H-IIL instruction with no return value
pub enum OpVoid {
    OpNop(instruction::OpNop),
}

pub enum Expr {
    Op(OpExpr),
    // ... constant, etc ...
}

impl Expr {
    fn flatten(self, ctx: &mut block::Ctx) -> BlockLocalExpr {
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
    use crate::iil::{self, block};

    pub struct OpNop;

    pub struct OpIAdd {
        pub r#type: iil::IntegerType,
        pub lhs: Box<iil::h::Expr>,
        pub rhs: Box<iil::h::Expr>,
    }
    impl super::FlattenableToBlock for OpIAdd {
        fn flatten(self, ctx: &mut block::Ctx) -> iil::h::Block {
            ctx.new_block(|b, ctx| {
                let lhs = b.push_valued_local(self.lhs.flatten(ctx));
                let rhs = b.push_valued_local(self.rhs.flatten(ctx));
                iil::f::OpExpr::OpIAdd(iil::f::instruction::OpIAdd {
                    r#type: self.r#type,
                    lhs,
                    rhs,
                })
            })
        }
    }
}
