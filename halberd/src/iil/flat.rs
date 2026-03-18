use enum_dispatch::enum_dispatch;

use crate::iil::{self, block};

#[enum_dispatch]
/// all F-IIL instructions
pub trait Op {}

/// an F-IIL instruction with a return value
pub enum OpExpr {
    OpIAdd(instruction::OpIAdd),
}

/// an F-IIL instruction with no return value
pub enum OpVoid {
    OpNop(instruction::OpNop),
}

pub enum Expr {
    Op(OpExpr),
}

pub type BlockVoidLocal = OpVoid;
pub type BlockValuedLocal = OpExpr;
pub type BlockLocal = block::BlockLocal<BlockVoidLocal, BlockValuedLocal>;

pub mod instruction {
    use crate::iil::{self, block};

    pub struct OpNop;
    impl iil::f::Op for OpNop {}

    pub struct OpIAdd {
        pub r#type: iil::IntegerType,
        pub lhs: block::BlockLocalRef,
        pub rhs: block::BlockLocalRef,
    }
    impl iil::f::Op for OpIAdd {}
}
