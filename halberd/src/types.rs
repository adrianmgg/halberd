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

impl Matrix {
    // not strictly needed but good to be consistent w/ `row_count`, plus it's inlined anyways so whatever
    #[inline(always)]
    pub fn column_count(&self) -> u32 {
        self.column_count
    }
    #[inline(always)]
    pub fn row_count(&self) -> u32 {
        self.column_type.component_count
    }
    #[inline(always)]
    pub fn component_type(&self) -> NumberKind {
        self.column_type.component_type
    }
}

macro_rules! mk_option_helper_exts {
    (
        $(
            $extname:ident ($ext_target:ty) {
                $( $method:ident $(( $($arg:ident : $argty:ty),* ))? -> $result:ty = $self:pat => { $($body:tt)* } )*
            };
        )*
    ) => {
        $(
            pub trait $extname: Sized {
                $( fn $method(self $( $( , $arg: $argty )* )?) -> Option<$result> ; )*
            }
            impl $extname for Option<$ext_target> {
                $( fn $method(self $( $( , $arg: $argty )* )?) -> Option<$result> {
                    match self {
                        Some($self) => { $($body)* }
                        None => None,
                    }
                } )*
            }
            impl $extname for $ext_target {
                $( fn $method(self $($(, $arg: $argty)*)?) -> Option<$result> {
                    match self {
                        $self => { $($body)* }
                    }
                } )*
            }
        )*
    };
}

pub mod prelude {
    use crate::util::matches_opt;

    use super::*;

    mk_option_helper_exts! {
        ExtTwoTypes((Type, Type)) {
            and_is_homogeneous -> Type = (lhs, rhs) => { (lhs == rhs).then_some(lhs) }
        };
        ExtAnyType(Type) {
            and_is_vector -> Vector = t => { matches_opt!(t, Type::Vector(v) => v) }
            and_is_matrix -> Matrix = t => { matches_opt!(t, Type::Matrix(m) => m) }
        };
        ExtVector(Vector) {
            // FIXME naming for `and_to_component_type`
            and_to_component_type -> NumberKind = v => { Some(v.component_type) }
            and_has_n_components(n: u32) -> Vector = v => { (v.component_count == n).then_some(v) }
        };
        ExtMatrix(Matrix) {
            to_component_type -> NumberKind = m => { Some(m.column_type.component_type) }
        };
        ExtNumberKind(NumberKind) {
            and_is_float -> Float = n => { matches_opt!(n, NumberKind::Float(f) => f) }
            and_is_int -> Integer = n => { matches_opt!(n, NumberKind::Integer(i) => i) }
        };
    }
}
