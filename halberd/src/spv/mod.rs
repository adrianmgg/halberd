pub use crate::generated::spv::{OpRetTyped, OpRetUntyped, OpVoid};

pub trait HasCapabilities {
    fn capabilities(&self) -> enumset::EnumSet<operand_kind::Capability>;
}

pub(crate) mod writer;

pub trait Instruction: HasCapabilities {
    fn opcode(&self) -> u32;
}

pub mod operand_kind {
    use num_bigint::{BigInt, BigUint};
    use num_rational::BigRational;
    use num_traits::One;

    pub use crate::generated::spv::operand_kind::*;
    use crate::{
        spv::writer::SpvWritable,
        types,
        util::{impl_conversion_2_hop, impl_conversion_enum_variant},
    };

    // FIXME these fields should probably be pub right
    #[derive(Debug)]
    pub struct LiteralInteger {
        value: BigInt,
        r#type: types::Integer,
    }

    // NOTE could potentially write a generic one for all built in num types w/ num-traits,
    //      but that seems like it opens up too much opportunity for mistake...
    impl From<u32> for LiteralInteger {
        fn from(value: u32) -> Self {
            Self { value: value.into(), r#type: types::Integer::Unsigned(32) }
        }
    }

    impl SpvWritable for LiteralInteger {
        fn write_spv_to<W: super::writer::SpvWriter + ?Sized>(
            &self,
            writer: &mut W,
        ) -> Result<(), W::Error> {
            // > For a numeric literal, the lower-order words appear first.
            // > If a numeric type’s bit width is less than 32-bits,
            // >  the value appears in the low-order bits of the word,
            // >  and the high-order bits must be 0 for a floating-point type or integer type with Signedness of 0,
            // >  or sign extended for an integer type with a Signedness of 1
            // >  (similarly for the remaining bits of widths larger than 32 bits but not a multiple of 32 bits).
            // - SPIR-V spec, 2.2.1. Instructions

            use num_bigint::Sign;

            // TODO should probably special case this w/ some faster versions for most of the normal
            //      int types we expect to actually encounter

            // TODO should refactor all the checks here to not be panics

            let n_words = self.r#type.bit_width().div_ceil(32);
            let last_word_bits = match self.r#type.bit_width() % 32 {
                0 => 32,
                extra => extra,
            };

            if !self.r#type.is_signed() {
                assert!(
                    self.value.sign() != Sign::Minus,
                    "LiteralInteger is negative but its type was unsigned"
                );
            }

            let unsigned_bound = if self.r#type.is_signed() {
                if self.value.sign() == Sign::Minus {
                    BigUint::one() << (self.r#type.bit_width() - 1)
                } else {
                    (BigUint::one() << (self.r#type.bit_width() - 1)) - 1u64
                }
            } else {
                BigUint::one() << self.r#type.bit_width()
            };
            assert!(
                self.value.magnitude() <= &unsigned_bound,
                "LiteralInteger value to big for its type"
            );

            let (sign, mut digits) = self.value.to_u32_digits();
            while digits.len() < n_words as usize {
                digits.push(0u32);
            }
            if sign == Sign::Minus {
                for digit in &mut digits {
                    *digit ^= u32::MAX;
                }
            }

            for digit in digits {
                writer.write_word(digit)?;
            }

            Ok(())
        }
    }

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
    impl_conversion_2_hop!(BigRational => LiteralFloat => LiteralContextDependentNumber);

    // > A string is interpreted as a nul-terminated stream of characters.
    // > All string comparisons are case sensitive.
    // > The character set is Unicode in the UTF-8 encoding scheme.
    // > The UTF-8 octets (8-bit bytes) are packed four per word,
    // >  following the little-endian convention (i.e., the first octet is in the lowest-order 8 bits of the word).
    // > The final word contains the string’s nul-termination character (0),
    // >  and all contents past the end of the string in the final word are padded with 0.
    // - SPIR-V spec, 2.2.1. Instructions
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
