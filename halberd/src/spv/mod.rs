pub mod asm;

pub trait HasCapabilities {
    fn capabilities(&self) -> enumset::EnumSet<operand_kind::Capability>;
}

pub mod operand_kind {
    pub use crate::generated::spv::operand_kind::*;

    /// TODO
    pub struct LiteralInteger;
    /// TODO
    pub struct LiteralString;
    /// TODO
    pub struct LiteralFloat;
    /// TODO
    pub struct LiteralContextDependentNumber;
    /// TODO
    pub struct LiteralExtInstInteger;
    /// TODO
    pub struct LiteralSpecConstantOpInteger;
}
