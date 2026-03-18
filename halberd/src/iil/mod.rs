pub mod block;
pub mod flat;
pub mod hierarchical;

pub use flat as f;
pub use hierarchical as h;

macro_rules! mk_types {
    (
        $( $variant:ident ($name:ident) $body:tt ),* $(,)?
    ) => {
        #[derive(Debug, Clone)]
        pub enum Type {
            $( $variant($name) ),*
        }

        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct $name $body

            impl From<$name> for Type {
                fn from(value: $name) -> Self {
                    Self::$variant(value)
                }
            }
        )*
    };
}
mk_types! {
    Integer(IntegerType) {
        pub width: u32,
        pub signed: bool,
    },
    Float(FloatType) {
        pub width: u32,
    },
}
