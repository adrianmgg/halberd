pub mod asm;

pub use crate::generated::spv::*;

pub trait HasCapabilities {
    fn capabilities(&self) -> impl Iterator<Item = operand_kind::Capability>;
}
