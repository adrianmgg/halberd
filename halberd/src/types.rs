use crate::util::{impl_conversion_2_hop, impl_conversion_enum_variant};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Type {
    Number(NumberKind),
    Vector(Vector),
    Matrix(Matrix),
}

impl_conversion_enum_variant!(Type::Number(NumberKind));
impl_conversion_enum_variant!(Type::Vector);
impl_conversion_enum_variant!(Type::Matrix);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NumberKind {
    Integer(Integer),
    Float(Float),
}

impl_conversion_enum_variant!(NumberKind::Float);
impl_conversion_enum_variant!(NumberKind::Integer);
impl_conversion_2_hop!(Integer => NumberKind => Type);
impl_conversion_2_hop!(Float => NumberKind => Type);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Integer {
    Unsigned(u32),
    Signed(u32),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Float {
    pub width: u32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Vector {
    pub component_type: NumberKind,
    pub component_count: u32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Matrix {
    pub column_type: Vector,
    pub column_count: u32,
}
