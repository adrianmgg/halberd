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
