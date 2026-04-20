use std::fmt::Display;

use chumsky::prelude::*;
use num_bigint::BigInt;
use num_rational::BigRational;
use rstest::rstest;

use crate::{
    types,
    util::{impl_conversion_2_hop, impl_conversion_enum_variant},
};

// TODO: design decision - do we flatten these all out like e.g.
//         Let, If, Else, Ident(&str), OpAdd, OpSubtract, ...
//       or do we do sub-enums like
//         Keyword(Keyword), Ident(&str), Op(Op)

// TODO: maybe move tokens to their own file separate from lexer?

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Token<'src> {
    Keyword(Keyword),
    Symbol(Symbol),
    DollarIdent(&'src str),
    Ident(&'src str),
    Op { op: Op, lifts: usize },
    Parens(Vec<Spanned<Self>>),
    Braces(Vec<Spanned<Self>>),
    Number(Number),
    Type(types::Type),
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Keyword(keyword) => write!(f, "{keyword}"),
            Token::Symbol(symbol) => write!(f, "{symbol}"),
            Token::DollarIdent(ident) => write!(f, "${ident}"),
            Token::Ident(ident) => write!(f, "{ident}"),
            Token::Op { op, lifts } => {
                write!(f, "{op}")?;
                for _ in 0..(*lifts) {
                    write!(f, "^")?;
                }
                Ok(())
            }
            Token::Parens(tokens) => write!(f, "(...)"),
            Token::Braces(tokens) => write!(f, "{{...}}"),
            Token::Number(number) => number.fmt(f),
            Token::Type(r#type) => r#type.fmt(f),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Symbol {
    Semicolon,
    Colon,
    Equals,
    Comma,
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Symbol::Semicolon => ";",
            Symbol::Colon => ":",
            Symbol::Equals => "=",
            Symbol::Comma => ",",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Keyword {
    // FIXME either remove or stop using $fn
    Function,
    Let,
    If,
    Else,
    True,
    False,
}

impl Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Keyword::Function => write!(f, "fn"),
            Keyword::Let => write!(f, "let"),
            Keyword::If => write!(f, "if"),
            Keyword::Else => write!(f, "else"),
            Keyword::True => write!(f, "true"),
            Keyword::False => write!(f, "false"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Number {
    pub value: NumberValue,
    pub kind: Option<types::NumberKind>,
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            NumberValue::Int(big_int) => big_int.fmt(f)?,
            NumberValue::Float(ratio) => ratio.fmt(f)?,
        }
        if let Some(kind) = &self.kind {
            kind.fmt(f)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NumberValue {
    Int(BigInt),
    Float(BigRational),
}

impl_conversion_enum_variant!(NumberValue::Int(BigInt));
impl_conversion_enum_variant!(NumberValue::Float(BigRational));
impl_conversion_2_hop!(i32 => BigInt => NumberValue);
impl_conversion_2_hop!(u64 => BigInt => NumberValue);

// for conciseness, allow using a Keyword variant as a 1-element Token sequence
// containing the Token corresponding with itself,
// so that e.g. `just(Keyword::True)` works as a parser accepting only Token::Keyword(Keyword::True)
impl chumsky::container::OrderedSeq<'_, Token<'_>> for Keyword {}
impl<'me, 'src> chumsky::container::Seq<'me, Token<'src>> for Keyword {
    type Item<'a>
        = Token<'src>
    where Self: 'a;
    type Iter<'a>
        = std::iter::Once<Token<'src>>
    where Self: 'a;

    fn seq_iter(&self) -> Self::Iter<'me> { std::iter::once(Token::Keyword(*self)) }

    fn contains(&self, val: &Token<'src>) -> bool
    where Token<'src>: PartialEq {
        matches!(val, Token::Keyword(kwd) if kwd == self)
    }

    fn to_maybe_ref<'b>(item: Self::Item<'b>) -> chumsky::util::MaybeRef<'src, Token<'src>>
    where 'me: 'b {
        chumsky::util::Maybe::Val(item)
    }
}

// FIXME should prob macro this
impl chumsky::container::OrderedSeq<'_, Token<'_>> for Symbol {}
impl<'me, 'src> chumsky::container::Seq<'me, Token<'src>> for Symbol {
    type Item<'a>
        = Token<'src>
    where Self: 'a;
    type Iter<'a>
        = std::iter::Once<Token<'src>>
    where Self: 'a;

    fn seq_iter(&self) -> Self::Iter<'me> { std::iter::once(Token::Symbol(*self)) }

    fn contains(&self, val: &Token<'src>) -> bool
    where Token<'src>: PartialEq {
        matches!(val, Token::Symbol(kwd) if kwd == self)
    }

    fn to_maybe_ref<'b>(item: Self::Item<'b>) -> chumsky::util::MaybeRef<'src, Token<'src>>
    where 'me: 'b {
        chumsky::util::Maybe::Val(item)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Op {
    Add,
    Subtract,
    Multiply,
    Divide,
    DotProduct,
    CrossProduct,
    MatrixMultiply,
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Op::Add => "+",
            Op::Subtract => "-",
            Op::Multiply => "*",
            Op::Divide => "/",
            Op::DotProduct => "*.",
            Op::CrossProduct => "*><",
            Op::MatrixMultiply => "*@",
        };
        write!(f, "{s}")
    }
}

type LexExtra<'src> = chumsky::extra::Err<Rich<'src, char, SimpleSpan>>;

pub fn lexer<'src>() -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, LexExtra<'src>> {
    let comment = just("$.")
        .then(any().and_is(just('\n').not()).repeated())
        .padded()
        .ignored()
        .labelled("comment");

    let token_top_level = recursive(|token| {
        let dollar_ident = just('$')
            .ignore_then(text::unicode::ident())
            .map(Token::DollarIdent)
            .labelled("dollar-keyword");

        let vaguely_ident_shaped = text::unicode::ident();

        let r#type = r#type()
            // ensure that there's no stray ident-like characters at the end of this thing that
            // looks like a type
            // (`i32foobar` is just a single ident of its own, not a type followed by an ident)
            .then_ignore(vaguely_ident_shaped.not())
            .map(Token::Type);

        let ident = vaguely_ident_shaped
            .map(|ident| match ident {
                "fn" => Token::Keyword(Keyword::Function),
                "let" => Token::Keyword(Keyword::Let),
                "if" => Token::Keyword(Keyword::If),
                "else" => Token::Keyword(Keyword::Else),
                "true" => Token::Keyword(Keyword::True),
                "false" => Token::Keyword(Keyword::False),
                other => Token::Ident(other),
            })
            .labelled("identifier or keyword");

        let sym = choice((
            just("=").to(Symbol::Equals),
            just(",").to(Symbol::Comma),
            just(";").to(Symbol::Semicolon),
            just(":").to(Symbol::Colon),
        ))
        .map(Token::Symbol)
        .labelled("non-operator symbol")
        .boxed();

        let op = choice((
            just('+').to(Op::Add),
            just('-').to(Op::Subtract),
            just("*.").to(Op::DotProduct),
            just("*><").to(Op::CrossProduct),
            just("*@").to(Op::MatrixMultiply),
            just('*').to(Op::Multiply),
            just('/').to(Op::Divide),
        ))
        .then(just('^').repeated().count())
        .map(|(op, lifts)| Token::Op { op, lifts })
        .labelled("operator");

        let maybe_token = comment.to(None).or(token.map(Some));

        let open_paren = just('(').labelled("open paren");
        let close_paren = just(')').labelled("closing paren");
        let open_brace = just('{').labelled("open brace");
        let close_brace = just('}').labelled("closing brace");

        let parens = maybe_token
            .clone()
            .repeated()
            .collect::<FilteredCollector<_>>()
            // NOTE: then_ignore(whitespace) is needed so e.g. `( )` doesn't fail to parse
            .delimited_by(open_paren.then_ignore(text::whitespace()), close_paren)
            .labelled("parenthesized tokens")
            .as_context()
            .map(FilteredCollector::inner)
            .map(Token::Parens);

        let braces = maybe_token
            .repeated()
            .collect::<FilteredCollector<_>>()
            .delimited_by(open_brace.then_ignore(text::whitespace()), close_brace)
            .labelled("braced tokens")
            .as_context()
            .map(FilteredCollector::inner)
            .map(Token::Braces);

        let any_token = choice((
            //
            op,
            r#type,
            ident,
            dollar_ident,
            number_literal().map(Token::Number),
            parens,
            braces,
            sym,
        ));

        any_token.spanned().padded()
        // .recover_with(skip_then_retry_until(any().ignored(), end()))
    });
    let maybe_token = comment.to(None).or(token_top_level.map(Some));
    maybe_token
        .repeated()
        .collect()
        .map(FilteredCollector::inner)
}

#[cfg(test)]
mod test_lex {
    use std::assert_matches;

    use chumsky::{Parser as _, span::Spanned};
    use rstest::rstest;

    use super::{Token, lexer};
    use crate::{lexer::Keyword, types};

    macro_rules! lex_test {
        ($name:ident, $s:literal, $m:pat $(if $guard:expr)?) => {
            #[test]
            fn $name() {
                let result = lexer().parse($s).into_result();
                // FIXME use unstable `assert_matches`
                assert!(matches!(result.as_deref(), $m $(if $guard)?), "got: {:?}", &result);
            }
        };
    }
    macro_rules! lex_test_single {
        ($name:ident, $s:literal, $m:pat $(if $guard:expr)?) => {
            lex_test!($name, $s, Ok([Spanned { inner: $m , .. }]) $(if $guard)?);
        };
    }

    lex_test!(just_a_comment, "$. hello world", Ok([]));
    lex_test!(
        comment_after,
        "a$. hello world",
        Ok([Spanned { inner: Token::Ident("a"), .. }])
    );
    lex_test!(
        comment_prev_line,
        "$. hello world\na",
        Ok([Spanned { inner: Token::Ident("a"), .. }])
    );
    lex_test!(
        multiple_tokens_top_level,
        "if if",
        Ok([
            Spanned { inner: Token::Keyword(Keyword::If), .. },
            Spanned { inner: Token::Keyword(Keyword::If), .. }
        ])
    );
    lex_test_single!(multiple_tokens_tt, "(if if)", Token::Parens(_));
    lex_test_single!(empty_tt, "()", Token::Parens(v) if v.is_empty());
    lex_test_single!(empty_tt_ws, "( )", Token::Parens(v) if v.is_empty());

    lex_test_single!(
        type_u32,
        "u32",
        Token::Type(t) if *t == types::Integer::Unsigned(32).into()
    );
    lex_test_single!(
        type_r64,
        "r64",
        Token::Type(t) if *t == types::Float { width: 64 }.into()
    );
    lex_test_single!(
        type_vec,
        "i32v99",
        Token::Type(t) if *t == types::Vector {
            component_type: types::NumberKind::Integer(types::Integer::Signed(32)),
            component_count: 99,
        }.into()
    );
    lex_test_single!(
        type_mat,
        "r32m12x34",
        Token::Type(t) if *t == types::Matrix {
            column_type: types::Vector {
                component_type: types::NumberKind::Float(types::Float { width: 32 }),
                component_count: 12,
            },
            column_count: 34,
        }.into()
    );
    // should lex as a single token [Ident(i32foo)], not as [Type(i32), Ident(foo)]
    lex_test_single!(ident_with_type_prefix, "i32foo", Token::Ident(_));
    lex_test_single!(ident_with_type_suffix, "fooi32", Token::Ident(_));
    lex_test_single!(ident_with_kwd_prefix, "iffoo", Token::Ident(_));
    lex_test_single!(ident_with_kwd_suffix, "fooif", Token::Ident(_));

    #[rstest]
    #[case::unmatched_paren_open("(")]
    #[case::unmatched_paren_close(")")]
    fn test_lex_fails(#[case] s: &'_ str) {
        assert_matches!(lexer().parse(s).into_result(), Err(_));
    }
}

fn int_parsed<'src, T>(radix: u32) -> impl Parser<'src, &'src str, T, LexExtra<'src>> + Copy
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    text::int(radix).try_map(|s: &str, span| {
        s.parse()
            .map_err(|err| Rich::custom(span, format!("invalid width: {err:?}")))
    })
}

fn number_kind<'src>() -> impl Parser<'src, &'src str, types::NumberKind, LexExtra<'src>> + Copy {
    let kind_width = int_parsed(10);
    choice((
        just('u')
            .ignore_then(kind_width)
            .map(|width| types::NumberKind::Integer(types::Integer::Unsigned(width))),
        just('i')
            .ignore_then(kind_width)
            .map(|width| types::NumberKind::Integer(types::Integer::Signed(width))),
        just('r')
            .ignore_then(kind_width)
            .map(|width| types::NumberKind::Float(types::Float { width })),
    ))
}

/// WARNING: this parser on its own will accept a type name which is part of an identifier, so
///          needs to be chained with other parsers to be fully correct in the context of the
///          overall lexer.
pub(crate) fn r#type<'src>() -> impl Parser<'src, &'src str, types::Type, LexExtra<'src>> + Clone {
    let n = int_parsed::<u32>(10);
    let vector_suffix = just('v').ignore_then(n);
    let matrix_suffix = just('m').ignore_then(n.then_ignore(just('x')).then(n));
    let nk = number_kind();
    choice((
        // e.g. 'i32v4'
        nk.then(vector_suffix)
            .map(|(component_type, component_count)| {
                types::Vector { component_type, component_count }.into()
            }),
        // e.g. 'i32m3x2'
        nk.then(matrix_suffix)
            .map(|(component_type, (component_count, column_count))| {
                types::Matrix {
                    column_type: types::Vector { component_type, component_count },
                    column_count,
                }
                .into()
            }),
        // e.g. 'i32'
        nk.map(Into::into),
    ))
    .boxed()
}

fn number_literal<'src>() -> impl Parser<'src, &'src str, Number, LexExtra<'src>> + Clone {
    // "1234x" -> 1234
    let radix_prefix = just('0')
        .repeated()
        .ignore_then(text::digits(10).to_slice())
        .then_ignore(just('x'))
        .validate(|s: &str, e, emitter| {
            let radix = s.parse::<u32>().unwrap_or_else(|err| {
                let msg = format!("invalid radix (failed to parse it as an int: {err:?})");
                emitter.emit(Rich::custom(e.span(), msg));
                // this isn't actually a *default* value, but it should make the parser able to
                // continue at least somewhat so that we can report this and any future errors.
                // FIXME should maybe be 16 instead
                10
            });
            if !(2..=16).contains(&radix) {
                emitter.emit(Rich::custom(e.span(), "radix out of range (not 2-16)"));
            }
            radix
        });

    // one digit w/ specified radix, or None on a '_' separator
    let digit_n = |radix| {
        any()
            .filter(move |c: &char| c.is_digit(radix) || matches!(c, '_'))
            .map(move |c| c.to_digit(radix))
    };
    // parses to (bigint value, number of digits)
    let n_n = move |radix| {
        digit_n(radix).repeated().at_least(1).fold(
            (BigInt::ZERO, 0u32),
            move |(acc, ndigits), c| match c {
                None => (acc, ndigits),
                Some(digit) => (acc * radix + digit, ndigits + 1),
            },
        )
    };

    custom(move |inp| {
        // optionally parse a radix prefix, or 10 if unspecified
        let radix_whole = inp.parse(radix_prefix.or(empty().to(10)))?;
        let (whole, _) = inp.parse(n_n(radix_whole))?;

        let dot = inp.parse(just('.').to(true).or(empty().to(false)))?;

        // fractional part, iif there was a dot
        let frac = dot
            .then(|| {
                // optionally parse a radix prefix, or the current radix if unspecified
                let radix_frac = inp.parse(radix_prefix.or(empty().to(radix_whole)))?;
                let frac = inp.parse(n_n(radix_frac))?;
                Ok((frac, radix_frac))
            })
            .transpose()?;

        let value = match frac {
            None => NumberValue::Int(whole),
            Some(((frac, ndigits), radix)) => {
                let n = BigRational::new(frac, BigInt::from(radix).pow(ndigits));
                NumberValue::Float(n + whole)
            }
        };

        let kind = inp.parse(number_kind().or_not())?;

        Ok(Number { value, kind })
    })
    .labelled("number literal")
}

#[rstest]
#[case::simple("1", Number { value: 1.into(), kind: None })]
#[case::float("1.2", Number { value: NumberValue::Float(BigRational::new(12.into(), 10.into())), kind: None })]
#[case::kindsuffix_uint("1u32", Number { value: 1.into(), kind: Some(types::NumberKind::Integer(types::Integer::Unsigned(32))) })]
#[case::kindsuffix_int("1i32", Number { value: 1.into(), kind: Some(types::NumberKind::Integer(types::Integer::Signed(32))) })]
#[case::kindsuffix_float("1r32", Number { value: 1.into(), kind: Some(types::NumberKind::Float(types::Float{width: 32})) })]
#[case::float_with_type("1.2r32", Number { value: NumberValue::Float(BigRational::new(12.into(), 10.into())), kind: Some(types::NumberKind::Float(types::Float{width: 32})) })]
#[case::underscores("1_2_3__4____5", Number { value: 12345.into(), kind: None })]
#[case::radix_simple("16xdead_beef", Number { value: 0xdead_beef_u64.into(), kind: None })]
#[case::radix_uniform("16xdead.beef", Number { value: NumberValue::Float(BigRational::new(3735928559u64.into(), 65536.into())), kind: None })]
#[case::radix_different("16xdead.10x1234", Number { value: NumberValue::Float(BigRational::new(285025617.into(), 5000.into())), kind: None })]
#[case::radix_different_implicitfirst("1234.16xbeef", Number { value: NumberValue::Float(BigRational::new(80920303.into(), 65536.into())), kind: None })]
fn test_lex_number(#[case] s: &'_ str, #[case] expected: Number) {
    assert_eq!(number_literal().parse(s).into_result(), Ok(expected));
}

#[derive(Clone)]
/// pretends to be a Vec<Option<T>>, but it only actually accepts the non-None values
struct FilteredCollector<T>(Vec<T>);

impl<T> FilteredCollector<T> {
    fn inner(self) -> Vec<T> { self.0 }
}
impl<T> Default for FilteredCollector<T> {
    fn default() -> Self { Self(Vec::default()) }
}
impl<T> chumsky::container::Container<Option<T>> for FilteredCollector<T> {
    fn push(&mut self, item: Option<T>) {
        if let Some(item) = item {
            self.0.push(item);
        }
    }
}
