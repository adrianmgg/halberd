pub use crate::generated::spv::{OpRetTyped, OpRetUntyped, OpVoid};
use crate::spv::writer::{SpvWritable, SpvWriter};

pub trait HasCapabilities {
    fn capabilities(&self) -> enumset::EnumSet<operand_kind::Capability>;
}

pub mod operand_kind;
pub(crate) mod writer;

pub trait Instruction: HasCapabilities {
    fn opcode(&self) -> u32;
    /// essentially a [`writer::SpvWritable::write_spv_to`] implementation, but only writes the non-
    /// -result-related operands (ie. neither its opcode nor its result type id or result id if any)
    fn write_operands_to(&self, writer: &mut dyn SpvWriter) -> writer::Result<()>;
}

pub trait InstructionRetTyped: Instruction {
    fn ret_type(&self) -> &operand_kind::IdResultType;
}

impl OpVoid {
    fn write_instruction(&self, writer: &mut dyn SpvWriter) -> writer::Result<()> {
        let inst = self.as_dyn_instruction();
        // opcode
        writer.write_word(inst.opcode())?;
        // ...operands
        inst.write_operands_to(writer)
    }
}

impl OpRetUntyped {
    fn write_instruction(
        &self,
        writer: &mut dyn SpvWriter,
        result_id: operand_kind::IdResult,
    ) -> writer::Result<()> {
        let inst = self.as_dyn_instruction();
        // opcode
        writer.write_word(inst.opcode())?;
        // Result <id>
        result_id.write_spv_to(writer)?;
        // ...operands
        inst.write_operands_to(writer)
    }
}

impl OpRetTyped {
    fn write_instruction(
        &self,
        writer: &mut dyn SpvWriter,
        result_id: operand_kind::IdResult,
    ) -> writer::Result<()> {
        let inst = self.as_dyn_instruction();
        // opcode
        writer.write_word(inst.opcode())?;
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
