use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use crate::util::{
    impl_conversion_2_hop, impl_conversion_copy_deref, impl_conversion_enum_variant,
    impl_debug_via_display, impl_display_enum_variants_transparent,
};

// FIXME can't currently represent boolean vectors

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum Type {
    Concrete(Concrete),
    Abstract(Abstract),
    Function(Function),
}
impl_conversion_enum_variant!(Type::{Concrete, Abstract});

impl_display_enum_variants_transparent!(Type { Concrete, Abstract, Function });
impl_debug_via_display!(Type);

// "Concrete Type: A numerical scalar, vector, or matrix type, or physical pointer type, or any aggregate containing only these types."
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum Concrete {
    Number(NumberKind),
    Vector(Vector),
    Matrix(Matrix),
}
impl_conversion_copy_deref!(Concrete);
impl_conversion_enum_variant!(Concrete::{Number(NumberKind), Vector, Matrix});
impl_conversion_2_hop!(NumberKind => Concrete => Type);
impl_conversion_2_hop!(Vector => Concrete => Type);
impl_conversion_2_hop!(Matrix => Concrete => Type);

impl_display_enum_variants_transparent!(Concrete { Number, Vector, Matrix });
impl_debug_via_display!(Concrete);

// "Abstract Type: An OpTypeVoid or OpTypeBool, or logical pointer type, or any aggregate type containing any of these."
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum Abstract {
    Void(Void),
    Bool(Bool),
}
impl_conversion_copy_deref!(Abstract);
impl_display_enum_variants_transparent!(Abstract { Bool, Void });
impl_conversion_enum_variant!(Abstract::{Void, Bool});
impl_conversion_2_hop!(Void => Abstract => Type);
impl_conversion_2_hop!(Bool => Abstract => Type);

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Bool;

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Void;

impl Display for Bool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "bool") }
}

impl Display for Void {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "void") }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Function {
    pub args: Vec<Type>,
    pub result: Box<Type>,
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "$fn(")?;
        for arg in &self.args {
            write!(f, "{arg}, ")?;
        }
        write!(f, "): {}", self.result)
    }
}
impl_debug_via_display!(Function);

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum FunctionResult {
    Concrete(Concrete),
    Abstract(Abstract),
}
impl_conversion_enum_variant!(FunctionResult::{Concrete, Abstract});

impl_display_enum_variants_transparent!(FunctionResult { Concrete, Abstract });

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum NumberKind {
    Integer(Integer),
    Float(Float),
}
impl_conversion_enum_variant!(NumberKind::{Float, Integer});
impl_conversion_copy_deref!(NumberKind);

impl_display_enum_variants_transparent!(NumberKind { Integer, Float });
impl_debug_via_display!(NumberKind);

impl_conversion_2_hop!(Integer => NumberKind => Type);
impl_conversion_2_hop!(Float => NumberKind => Type);
impl_conversion_copy_deref!(Integer);
impl_conversion_copy_deref!(Float);
impl_conversion_2_hop!(&Integer => Integer => Type);
impl_conversion_2_hop!(&Float => Float => Type);

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum Integer {
    Unsigned(u32),
    Signed(u32),
}

impl Display for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Integer::Unsigned(width) => write!(f, "u{width}"),
            Integer::Signed(width) => write!(f, "i{width}"),
        }
    }
}
impl_debug_via_display!(Integer);

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Float {
    pub width: u32,
}

impl Display for Float {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r{}", self.width)
    }
}
impl_debug_via_display!(Float);

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Vector {
    pub component_type: NumberKind,
    pub component_count: u32,
}

impl Display for Vector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}v{}", self.component_type, self.component_count)
    }
}
impl_debug_via_display!(Vector);

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Matrix {
    pub column_type: Vector,
    pub column_count: u32,
}

impl Matrix {
    // not strictly needed but good to be consistent w/ `row_count`, plus it's inlined anyways so whatever
    #[inline(always)]
    pub fn column_count(&self) -> u32 { self.column_count }

    #[inline(always)]
    pub fn row_count(&self) -> u32 { self.column_type.component_count }

    #[inline(always)]
    pub fn component_type(&self) -> NumberKind { self.column_type.component_type }
}

impl Display for Matrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}m{}x{}",
            self.component_type(),
            self.row_count(),
            self.column_count()
        )
    }
}
impl_debug_via_display!(Matrix);

macro_rules! mk_option_helper_exts {
    (
        $life:lifetime ;
        $(
            $extname:ident ($ext_target:ty) {
                $( $method:ident $(( $($arg:ident : $argty:ty),* ))? -> $result:ty = $self:pat => { $($body:tt)* } )*
                $(($target2:ty) {
                    $( $method2:ident $(( $($arg2:ident : $argty2:ty),* ))? -> $result2:ty = $self2:pat => { $($body2:tt)* } )*
                })*
            };
        )*
    ) => {
        $(
            pub trait $extname<$life>: Sized {
                $( fn $method(self $( $( , $arg: $argty )* )?) -> Option<$result> ; )*
            }
            impl<$life> $extname<$life> for Option<$ext_target> {
                $( fn $method(self $( $( , $arg: $argty )* )?) -> Option<$result> {
                    match self {
                        Some($self) => { $($body)* }
                        None => None,
                    }
                } )*
            }
            impl<$life> $extname<$life> for $ext_target {
                $( fn $method(self $($(, $arg: $argty)*)?) -> Option<$result> {
                    match self {
                        $self => { $($body)* }
                    }
                } )*
            }
            $( impl<$life> $extname<$life> for $target2 {
                $( fn $method2(self $($(, $arg2: $argty2)*)?) -> Option<$result2> {
                    match self {
                        $self2 => { $($body2)* }
                    }
                } )*
            } )*
        )*
    };
}

pub mod prelude {
    use super::*;
    use crate::util::matches_opt;

    mk_option_helper_exts! { 'a;
        ExtTwoTypes((&'a Type, &'a Type)) {
            and_is_homogeneous -> &'a Type = (t1, t2) => { (t1 == t2).then_some(t1) }
        };
        ExtAnyType(&'a Type) {
            and_is_vector -> &'a Vector = t => { matches_opt!(t, Type::Concrete(Concrete::Vector(v)) => v) }
            and_is_matrix -> &'a Matrix = t => { matches_opt!(t, Type::Concrete(Concrete::Matrix(m)) => m) }
            and_is_number -> &'a NumberKind = t => { matches_opt!(t, Type::Concrete(Concrete::Number(n)) => n) }
            (&'a Option<Type>) {
                and_is_vector -> &'a Vector = t => { matches_opt!(t.as_ref(), Some(Type::Concrete(Concrete::Vector(v))) => v) }
                and_is_matrix -> &'a Matrix = t => { matches_opt!(t.as_ref(), Some(Type::Concrete(Concrete::Matrix(m))) => m) }
                and_is_number -> &'a NumberKind = t => { matches_opt!(t.as_ref(), Some(Type::Concrete(Concrete::Number(n))) => n) }
            }
        };
        ExtVector(&'a Vector) {
            // FIXME naming for `and_to_component_type`
            and_to_component_type -> &'a NumberKind = v => { Some(&v.component_type) }
            and_has_n_components(n: u32) -> &'a Vector = v => { (v.component_count == n).then_some(v) }
        };
        ExtMatrix(&'a Matrix) {
            to_component_type -> NumberKind = m => { Some(m.column_type.component_type) }
        };
        ExtNumberKind(&'a NumberKind) {
            and_is_float -> &'a Float = n => { matches_opt!(n, NumberKind::Float(f) => f) }
            and_is_int -> &'a Integer = n => { matches_opt!(n, NumberKind::Integer(i) => i) }
        };
    }
}

mod to_spv {
    use super::*;
    use crate::spv::{self, instruction as inst, operand_kind as ok};

    trait TypeToSpv {
        fn prerequisites(&self) -> Box<dyn Iterator<Item = Type>>;
        fn to_direct_instruction(
            &self,
            available: HashMap<Type, ok::IdRef>,
        ) -> Option<spv::OpRetUntyped>;
    }

    macro_rules! impl_type_to_spv {
        (
            $self:ident; $available:ident;
            $( $target:ty {
                prereq { $req_impl:expr }
                to { $to_impl:expr }
            } )*
        ) => {
            $(impl TypeToSpv for $target {
                fn prerequisites(&$self) -> Box<dyn Iterator<Item = Type>> { $req_impl }
                fn to_direct_instruction(
                    &$self,
                    $available: HashMap<Type, ok::IdRef>,
                ) -> Option<spv::OpRetUntyped> {
                    try { $to_impl }
                }
            })*
        };
        (@dispatch_variants $target:ty { $($variant:ident),* }) => {
            impl TypeToSpv for $target {
                fn prerequisites(&self) -> Box<dyn Iterator<Item = Type>> {
                    match self {
                        $( Self::$variant(x) => x.prerequisites() ),*
                    }
                }
                fn to_direct_instruction(
                    &self,
                    available: HashMap<Type, ok::IdRef>,
                ) -> Option<spv::OpRetUntyped> {
                    match self {
                        $( Self::$variant(x) => x.to_direct_instruction(available) ),*
                    }
                }
            }
        };
    }

    use std::iter::{chain, empty, once};

    // FIXME there's a bunch of unnecissary inefficiency in how we're doing the prereqs here atm.
    impl_type_to_spv! { self; available;
        Matrix {
            prereq { Box::new(once(self.column_type.into())) }
            to { spv::instruction::OpTypeMatrix {
                op0: *available.get(&self.column_type.into())?,
                op1: self.column_count.into(),
            }.into() }
        }
        Vector {
            prereq { Box::new(once(self.component_type.into())) }
            to { spv::instruction::OpTypeVector {
                op0: *available.get(&self.component_type.into())?,
                op1: self.component_count.into(),
            }.into() }
        }
        Integer {
            prereq { Box::new(empty()) }
            to { match self {
                Self::Signed(width) => spv::instruction::OpTypeInt { op0: (*width).into(), op1: 1.into() }.into(),
                Self::Unsigned(width) => spv::instruction::OpTypeInt { op0: (*width).into(), op1: 0.into() }.into(),
            } }
        }
        Float {
            prereq { Box::new(empty()) }
            to { inst::OpTypeFloat {op0: self.width.into(), op1: None }.into() }
        }
        Void {
            prereq { Box::new(empty()) }
            to { inst::OpTypeVoid {}.into() }
        }
        Bool {
            prereq { Box::new(empty()) }
            to { inst::OpTypeBool {}.into() }
        }
        Function {
            prereq { Box::new(chain(
                once(*self.result.clone()),
                self.args.clone().into_iter()
            )) }
            to { inst::OpTypeFunction {
                op0: *available.get(&self.result)?,
                op1: self.args.iter().map(|arg| available.get(arg).copied()).try_collect()?,
            }.into() }
        }
    }
    impl_type_to_spv! {@dispatch_variants Type { Concrete, Abstract, Function }}
    impl_type_to_spv! {@dispatch_variants Concrete { Number, Vector, Matrix }}
    impl_type_to_spv! {@dispatch_variants Abstract { Void, Bool }}
    impl_type_to_spv! {@dispatch_variants NumberKind { Integer, Float }}
}
