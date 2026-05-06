pub use crate::generated::spv::{
    MAGIC, MAJOR_VERSION, MINOR_VERSION, OpRetTyped, OpRetUntyped, OpVoid,
};
use crate::spv::writer::{SpvWritable, SpvWriter};

/// magic number is ascii `amgg` XOR `halb` XOR `erdc` ("amgg halberdc")
pub const GENERATOR_MAGIC: u32 = u32::from_be_bytes([
    b'a' ^ b'h' ^ b'e',
    b'm' ^ b'a' ^ b'r',
    b'g' ^ b'l' ^ b'd',
    b'g' ^ b'b' ^ b'c',
]);

pub trait HasCapabilities {
    fn capabilities(&self) -> enumset::EnumSet<operand_kind::Capability>;
}

pub mod operand_kind;
pub(crate) mod writer;

pub trait Instruction: HasCapabilities {
    fn opcode(&self) -> u16;
    /// essentially a [`writer::SpvWritable::write_spv_to`] implementation, but only writes the non-
    /// -result-related operands (ie. neither its opcode nor its result type id or result id if any)
    fn write_operands_to(&self, writer: &mut dyn SpvWriter) -> writer::Result<()>;
    /// word-length of operands
    fn tell_operands(&self) -> u16;
}

pub trait InstructionRetTyped: Instruction {
    fn ret_type(&self) -> &operand_kind::IdResultType;
}

impl OpVoid {
    pub fn write_instruction(&self, writer: &mut dyn SpvWriter) -> writer::Result<()> {
        let inst = self.as_dyn_instruction();
        let word_count = inst.tell_operands() + 1;
        // opcode
        writer.write_word((u32::from(word_count) << u16::BITS) | u32::from(inst.opcode()))?;
        // ...operands
        inst.write_operands_to(writer)
    }
}

impl OpRetUntyped {
    pub fn write_instruction(
        &self,
        writer: &mut dyn SpvWriter,
        result_id: operand_kind::IdResult,
    ) -> writer::Result<()> {
        let inst = self.as_dyn_instruction();
        let word_count = inst.tell_operands() + 2;
        // opcode
        writer.write_word((u32::from(word_count) << u16::BITS) | u32::from(inst.opcode()))?;
        // Result <id>
        result_id.write_spv_to(writer)?;
        // ...operands
        inst.write_operands_to(writer)
    }
}

impl OpRetTyped {
    pub fn write_instruction(
        &self,
        writer: &mut dyn SpvWriter,
        result_id: operand_kind::IdResult,
    ) -> writer::Result<()> {
        let inst = self.as_dyn_instruction();
        let word_count = inst.tell_operands() + 3;
        // opcode
        writer.write_word((u32::from(word_count) << u16::BITS) | u32::from(inst.opcode()))?;
        // <id> Result Type
        inst.ret_type().write_spv_to(writer)?;
        // Result <id>
        result_id.write_spv_to(writer)?;
        // ...operands
        inst.write_operands_to(writer)
    }
}

pub mod instruction {
    pub use crate::generated::spv::instruction::*;
}
