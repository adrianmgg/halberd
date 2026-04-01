macro_rules! impl_conversion_enum_variant {
    ($outer:ident :: $inner:ident) => {
        $crate::util::impl_conversion_enum_variant! {$outer :: $inner ( $inner )}
    };
    ($outer:ident :: $variant:ident ($inner:ident)) => {
        impl From<$inner> for $outer {
            fn from(x: $inner) -> $outer {
                $outer::$variant(x)
            }
        }
    };
}

pub(crate) use impl_conversion_enum_variant;

macro_rules! impl_conversion_2_hop {
    ($start:ty => $via:ty => $end:ty) => {
        impl From<$start> for $end {
            fn from(x: $start) -> $end {
                <$end>::from(<$via>::from(x))
            }
        }
    };
}

pub(crate) use impl_conversion_2_hop;
