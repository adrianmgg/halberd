pub use crate::generated::spv::{OpRetTyped, OpRetUntyped, OpVoid};

pub trait HasCapabilities {
    fn capabilities(&self) -> enumset::EnumSet<operand_kind::Capability>;
}

pub trait Instruction: HasCapabilities {
    fn opcode(&self) -> u32;
}

pub mod operand_kind {
    use num_bigint::BigInt;
    use num_rational::BigRational;

    pub use crate::generated::spv::operand_kind::*;
    use crate::util::{impl_conversion_2_hop, impl_conversion_enum_variant};

    #[derive(Debug)]
    pub struct LiteralInteger {
        value: BigInt,
    }
    impl From<BigInt> for LiteralInteger {
        fn from(value: BigInt) -> Self { Self { value } }
    }
    impl_conversion_2_hop!(u32 => BigInt => LiteralInteger);
    impl_conversion_2_hop!(u64 => BigInt => LiteralInteger);
    impl_conversion_2_hop!(i32 => BigInt => LiteralInteger);
    impl_conversion_2_hop!(i64 => BigInt => LiteralInteger);

    #[derive(Debug)]
    pub struct LiteralFloat {
        value: BigRational,
    }
    impl From<BigRational> for LiteralFloat {
        fn from(value: BigRational) -> Self { Self { value } }
    }

    #[derive(Debug)]
    pub enum LiteralContextDependentNumber {
        Integer(LiteralInteger),
        Float(LiteralFloat),
    }
    impl_conversion_enum_variant!(LiteralContextDependentNumber::{Integer(LiteralInteger), Float(LiteralFloat)});
    impl_conversion_2_hop!(BigInt => LiteralInteger => LiteralContextDependentNumber);
    impl_conversion_2_hop!(BigRational => LiteralFloat => LiteralContextDependentNumber);

    /// TODO
    #[derive(Debug)]
    pub struct LiteralString;
    /// TODO
    #[derive(Debug)]
    pub struct LiteralExtInstInteger;
    /// TODO
    #[derive(Debug)]
    pub struct LiteralSpecConstantOpInteger;
}

pub mod instruction {
    pub use crate::generated::spv::instruction::*;
}
