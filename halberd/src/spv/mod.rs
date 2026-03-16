pub trait HasCapabilities {
    fn capabilities(&self) -> enumset::EnumSet<operand_kind::Capability>;
}

pub mod operand_kind {
    pub use crate::generated::spv::operand_kind::*;

    /// TODO
    #[derive(Debug)]
    pub struct LiteralInteger;
    /// TODO
    #[derive(Debug)]
    pub struct LiteralString;
    /// TODO
    #[derive(Debug)]
    pub struct LiteralFloat;
    /// TODO
    #[derive(Debug)]
    pub struct LiteralContextDependentNumber;
    /// TODO
    #[derive(Debug)]
    pub struct LiteralExtInstInteger;
    /// TODO
    #[derive(Debug)]
    pub struct LiteralSpecConstantOpInteger;
}
