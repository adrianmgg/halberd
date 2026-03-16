pub mod asm;

pub use crate::generated::spv::*;

pub trait HasCapabilities {
    fn capabilities(&self) -> enumset::EnumSet<operand_kind::Capability>;
}
