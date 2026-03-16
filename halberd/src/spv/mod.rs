pub mod asm;

pub trait HasCapabilities {
    fn capabilities(&self) -> enumset::EnumSet<operand_kind::Capability>;
}

pub mod operand_kind {
    pub use crate::generated::spv::operand_kind::*;

    pub struct LiteralInteger;
}
