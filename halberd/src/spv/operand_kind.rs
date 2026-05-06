use num_bigint::{BigInt, BigUint};
use num_rational::BigRational;
use num_traits::One;

pub use crate::generated::spv::operand_kind::*;
use crate::{
    spv::{
        self,
        writer::{SpvWritable, SpvWriter},
    },
    types,
    util::{impl_conversion_2_hop, impl_conversion_enum_variant},
};

// FIXME these fields should probably be pub right
#[derive(Debug, Clone)]
pub struct LiteralInteger {
    pub value: BigInt,
    pub r#type: types::Integer,
}

// NOTE could potentially write a generic one for all built in num types w/ num-traits,
//      but that seems like it opens up too much opportunity for mistake...
impl From<u32> for LiteralInteger {
    fn from(value: u32) -> Self {
        Self { value: value.into(), r#type: types::Integer::Unsigned(32) }
    }
}

impl SpvWritable for LiteralInteger {
    fn write_spv_to(&self, writer: &mut dyn SpvWriter) -> spv::writer::Result<()> {
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

    fn tell_spv_wordcount(&self) -> u16 { self.r#type.bit_width().div_ceil(32) as u16 }
}

#[derive(Debug, Clone)]
pub struct LiteralFloat {
    pub value: BigRational,
    pub r#type: types::Float,
}

impl SpvWritable for LiteralFloat {
    fn write_spv_to(&self, writer: &mut dyn SpvWriter) -> spv::writer::Result<()> { todo!() }

    fn tell_spv_wordcount(&self) -> u16 { todo!() }
}

#[derive(Debug, Clone)]
pub enum LiteralContextDependentNumber {
    Integer(LiteralInteger),
    Float(LiteralFloat),
}
impl_conversion_enum_variant!(LiteralContextDependentNumber::{Integer(LiteralInteger), Float(LiteralFloat)});

impl SpvWritable for LiteralContextDependentNumber {
    fn write_spv_to(&self, writer: &mut dyn SpvWriter) -> spv::writer::Result<()> {
        match self {
            Self::Integer(i) => i.write_spv_to(writer),
            Self::Float(f) => f.write_spv_to(writer),
        }
    }

    fn tell_spv_wordcount(&self) -> u16 {
        match self {
            Self::Integer(i) => i.tell_spv_wordcount(),
            Self::Float(f) => f.tell_spv_wordcount(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiteralString {
    pub value: Box<str>,
}

impl SpvWritable for LiteralString {
    fn write_spv_to(&self, writer: &mut dyn SpvWriter) -> spv::writer::Result<()> {
        // > A string is interpreted as a nul-terminated stream of characters.
        // > All string comparisons are case sensitive.
        // > The character set is Unicode in the UTF-8 encoding scheme.
        // > The UTF-8 octets (8-bit bytes) are packed four per word,
        // >  following the little-endian convention (i.e., the first octet is in the lowest-order 8 bits of the word).
        // > The final word contains the string’s nul-termination character (0),
        // >  and all contents past the end of the string in the final word are padded with 0.
        // - SPIR-V spec, 2.2.1. Instructions

        let mut wrote_any_chunk = false;
        let mut wrote_unaligned_chunk = false;
        let bytes = self.value.as_bytes();
        for chunk in bytes.chunks(4) {
            assert!(!wrote_unaligned_chunk);
            let word = if let [a, b, c, d] = chunk[..] {
                u32::from_le_bytes([a, b, c, d])
            } else {
                wrote_unaligned_chunk = true;
                match chunk[..] {
                    [a, b, c] => u32::from_le_bytes([a, b, c, 0]),
                    [a, b] => u32::from_le_bytes([a, b, 0, 0]),
                    [a] => u32::from_le_bytes([a, 0, 0, 0]),
                    _ => unreachable!(
                        "should only be able to get chunks of size 1, 2, or 3 here, but got a chunk of size {}",
                        chunk.len()
                    ),
                }
            };
            writer.write_word(word)?;
            wrote_any_chunk = true;
        }
        if !wrote_any_chunk || !wrote_unaligned_chunk {
            writer.write_word(0)?;
        }

        Ok(())
    }

    fn tell_spv_wordcount(&self) -> u16 {
        let byte_len = self.value.len();
        let content_word_len = byte_len.div_floor(4) as u16;
        if byte_len == 0 || byte_len.is_multiple_of(4) {
            content_word_len + 1
        } else {
            content_word_len
        }
    }
}

/// TODO
#[derive(Debug, Clone)]
pub struct LiteralExtInstInteger;

impl SpvWritable for LiteralExtInstInteger {
    fn write_spv_to(&self, writer: &mut dyn SpvWriter) -> spv::writer::Result<()> { todo!() }

    fn tell_spv_wordcount(&self) -> u16 { todo!() }
}

/// TODO
#[derive(Debug, Clone)]
pub struct LiteralSpecConstantOpInteger;

impl SpvWritable for LiteralSpecConstantOpInteger {
    fn write_spv_to(&self, writer: &mut dyn SpvWriter) -> spv::writer::Result<()> { todo!() }

    fn tell_spv_wordcount(&self) -> u16 { todo!() }
}
