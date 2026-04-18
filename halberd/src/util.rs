/// ```rust
/// struct Foo;
/// struct Abcd;
/// enum Bar {
///     Foo(Foo),
///     Qux(Abcd),
/// }
/// impl_conversion_enum_variant!(Bar::Foo);
/// impl_conversion_enum_variant!(Bar::Qux(Abcd));
/// ```
macro_rules! impl_conversion_enum_variant {
    ($outer:ident:: $inner:ident) => {
        $crate::util::impl_conversion_enum_variant! {$outer :: $inner ( $inner )}
    };
    ($outer:ident:: $variant:ident($inner:ident)) => {
        impl From<$inner> for $outer {
            fn from(x: $inner) -> $outer { $outer::$variant(x) }
        }
    };
}
pub(crate) use impl_conversion_enum_variant;

macro_rules! impl_conversion_2_hop {
    ($start:ty => $via:ty => $end:ty) => {
        impl From<$start> for $end {
            fn from(x: $start) -> $end { <$end>::from(<$via>::from(x)) }
        }
    };
}
pub(crate) use impl_conversion_2_hop;

macro_rules! impl_conversion_copy_deref {
    ($ty:ty) => {
        impl From<&$ty> for $ty {
            #[inline(always)]
            fn from(x: &$ty) -> $ty { *x }
        }
    };
}
pub(crate) use impl_conversion_copy_deref;

/// helper to create a Debug impl using an existing Display impl
macro_rules! impl_debug_via_display {
    ($t:ty) => {
        impl ::std::fmt::Debug for $t {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_fmt(format_args!("{self}"))
            }
        }
    };
}
pub(crate) use impl_debug_via_display;

/// ```rust
/// enum A {
///     X(u32),
///     Y(bool),
/// }
/// let a = A::X(5);
/// let b = A::Y(true);
/// assert_eq!(matches_opt!(a, A::X(n) => n), Some(5));
/// assert_eq!(matches_opt!(b, A::X(n) => n), None);
/// ```
macro_rules! matches_opt {
    ( $expr:expr, $pat:pat $(if $guard:expr)? => $to:expr) => {
        match $expr {
            $pat $(if $guard)? => Some($to),
            _ => None,
        }
    };
}
pub(crate) use matches_opt;

/// make a [Display] implementation by directly calling thru to an enum's variants' [Display] impls
macro_rules! impl_display_enum_variants_transparent {
    ($enum:ty { $($variant:ident),* }) => {
        impl ::std::fmt::Display for $enum {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    $(Self::$variant(x) => ::std::fmt::Display::fmt(x, f)),*
                }
            }
        }
    };
}
pub(crate) use impl_display_enum_variants_transparent;
