use crate::util::{
    impl_conversion_2_hop, impl_conversion_copy_deref, impl_conversion_enum_variant,
};

// FIXME can't currently represent boolean vectors

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Type {
    Void,
    Bool,
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
impl_conversion_copy_deref!(Integer);
impl_conversion_copy_deref!(Float);
impl_conversion_2_hop!(&Integer => Integer => Type);
impl_conversion_2_hop!(&Float => Float => Type);

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

macro_rules! mk_option_helper_exts {
    (
        $(
            $extname:ident ($ext_target:ty) {
                $( $method:ident $(( $($arg:ident : $argty:ty),* ))? -> $result:ty = $self:tt => { $($body:tt)* } )*
            };
        )*
    ) => {
        $(
            pub trait $extname: Sized {
                $( fn $method(self $( $( , $arg: $argty )* )?) -> Option<$result> ; )*
            }
            impl $extname for Option<$ext_target> {
                $( fn $method($self $( $( , $arg: $argty )* )?) -> Option<$result> {
                    $($body)*
                } )*
            }
        )*
    };
}

pub mod prelude {
    use super::*;

    mk_option_helper_exts! {
        ExtTwoTypes((Type, Type)) {
            and_is_homogeneous -> Type = self => {self.and_then(|(lhs, rhs)| (lhs == rhs).then_some(lhs))}
        };
        ExtAnyType(Type) {
            and_is_vector -> Vector = self => { self.and_then(|t| match t {
                Type::Vector(v) => Some(v),
                _ => None,
            }) }
            and_is_matrix -> Matrix = self => { self.and_then(|t| match t {
                Type::Matrix(m) => Some(m),
                _ => None,
            }) }
        };
        ExtVector(Vector) {
            to_component_type -> NumberKind = self => { self.map(|v| v.component_type) }
            and_has_n_components(n: u32) -> Vector = self => { self.and_then(|v| (v.component_count == n).then_some(v)) }
        };
        ExtMatrix(Matrix) {
        };
        ExtNumberKind(NumberKind) {
            and_is_float -> Float = self => { self.and_then(|n| match n { NumberKind::Float(f) => Some(f), _ => None, }) }
            and_is_int -> Integer = self => { self.and_then(|n| match n { NumberKind::Integer(i) => Some(i), _ => None, }) }
        };
    }
}
