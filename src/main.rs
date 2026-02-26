pub(crate) mod lexer;
pub(crate) mod parser;
pub(crate) mod ast;

use chumsky::{Parser, error::Rich, pratt, prelude::*};

pub(crate) mod constants {
    pub(crate) mod op_power {
        pub(crate) const ADDSUB: u16 = 2;
        pub(crate) const MULDIV: u16 = 3;
        pub(crate) const LIFTED: u16 = 1;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Expr {
    LiteralInt(i64),
    InfixOp(Box<Expr>, InfixOpSimple, Box<Expr>),
}

macro_rules! mk_infix_ops {
        (
            $( $name:ident ( $token:literal, $associativity:ident, $power:ident ) ),* $(,)?
        ) => {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            enum InfixOpSimple {
                $( $name ),*
            }

            impl InfixOpSimple {
                fn tokenstr(&self) -> &'static str {
                    match self {
                        $( Self::$name => $token ),*
                    }
                }

                fn associativity(&self) -> ::chumsky::pratt::Associativity {
                    match self {
                        $( Self::$name => ::chumsky::pratt::$associativity ($crate::constants::op_power::$power) ),*
                    }
                }

                // TODO maybe do an iterator thing here instead
                fn all() -> &'static [Self] {
                    &[ $( Self::$name ),* ]
                }
            }
        };
    }
mk_infix_ops! {
    Add("+", left, ADDSUB),
    Subtract("-", left, ADDSUB),
    Multiply("*", left, MULDIV),
    Divide("/", left, MULDIV),
    DotProduct("*.", left, MULDIV),
    CrossProduct("*><", left, MULDIV),
    MatrixMultiply("*@", left, MULDIV),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct InfixOpFull {
    op: InfixOpSimple,
    lifted: bool,
}

impl InfixOpFull {
    fn tokenstr(&self) -> String {
        match self {
            Self { op, lifted: false } => op.tokenstr().into(),
            Self { op, lifted: true } => format!("{}^", op.tokenstr()),
        }
    }

    fn associativity(&self) -> pratt::Associativity {
        if self.lifted {
            pratt::left(constants::op_power::LIFTED)
        } else {
            self.op.associativity()
        }
    }

    fn all() -> impl Iterator<Item = Self> {
        let normals = InfixOpSimple::all().iter().map(|op| Self {
            op: *op,
            lifted: false,
        });
        let lifteds = InfixOpSimple::all().iter().map(|op| Self {
            op: *op,
            lifted: true,
        });
        lifteds.chain(normals)
    }
}

fn parser<'a>() -> impl Parser<'a, &'a str, Expr, chumsky::extra::Err<Rich<'a, char>>> {
    let atom = text::int(10)
        .from_str()
        .unwrapped()
        .map(Expr::LiteralInt)
        .padded();

    for op in InfixOpFull::all() {
        println!("{:?} ('{}') {:?}", op.op, op.tokenstr(), op.associativity());
    }

    atom.pratt(
        InfixOpFull::all()
            .map(|op| {
                pratt::infix(
                    op.associativity(),
                    just(op.tokenstr()).padded(),
                    move |l, _, r, _| Expr::InfixOp(Box::new(l), op.op, Box::new(r)),
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn main() {
    // let s = "1-2 *.^ 3-4 /^ 5";
    let s = "1 + 2 * 3";
    let e = parser().parse(s).into_result();
    dbg!(e);

    {
        macro_rules! E {
            ($i:literal) => {
                Expr::LiteralInt($i)
            };
            (($($e1:tt)*) $op:ident ($($e2:tt)*)) => {
                Expr::InfixOp(
                    Box::new( E!{$($e1)*} ),
                    InfixOpSimple::$op,
                    Box::new( E!{$($e2)*} ),
                )
            };
        }

        assert_eq!(
            parser().parse("1 + 2 * 3 + 4").into_result(),
            Ok(E!(  ( (1) Add ((2) Multiply (3)) ) Add (4)  )),
        );
        assert_eq!(
            parser().parse("1 + 2 *^ 3 + 4").into_result(),
            Ok(E!(  ((1) Add (2)) Multiply ((3) Add (4))  )),
        );

        assert_eq!(
            parser().parse("1 * 2 * 3").into_result(),
            Ok(E!(  ((1) Multiply (2)) Multiply (3)  )),
        );
        assert_eq!(
            parser().parse("1 *^ 2 * 3").into_result(),
            Ok(E!(  (1) Multiply ((2) Multiply (3))  )),
        );
    }
}
