use chumsky::prelude::*;

// TODO: design decision - do we flatten these all out like e.g.
//         Let, If, Else, Ident(&str), OpAdd, OpSubtract, ...
//       or do we do sub-enums like
//         Keyword(Keyword), Ident(&str), Op(Op)

// TODO: maybe move tokens to their own file separate from lexer?

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Token<'src> {
    Keyword(Keyword),
    DollarIdent(&'src str),
    Ident(&'src str),
    Op { op: Op, lifts: usize },
    Equals,
    Parens(Vec<Spanned<Self>>),
    Braces(Vec<Spanned<Self>>),
    Semicolon,
    Number(Number),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Keyword {
    Function,
    Let,
    If,
    Else,
    True,
    False,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Number {
    pub whole: u64,
    pub frac: Option<u64>,
    pub kind: Option<NumberKind>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NumberKind {
    Unsigned(u32),
    Signed(u32),
    Float(u32),
}

// for conciseness, allow using a Keyword variant as a 1-element Token sequence
// containing the Token corresponding with itself,
// so that e.g. `just(Keyword::True)` works as a parser accepting only Token::Keyword(Keyword::True)
impl<'src> chumsky::container::OrderedSeq<'_, Token<'src>> for Keyword {}
impl<'me, 'src> chumsky::container::Seq<'me, Token<'src>> for Keyword {
    type Item<'a>
        = Token<'src>
    where
        Self: 'a;

    type Iter<'a>
        = std::iter::Once<Token<'src>>
    where
        Self: 'a;

    fn seq_iter(&self) -> Self::Iter<'me> {
        std::iter::once(Token::Keyword(*self))
    }

    fn contains(&self, val: &Token<'src>) -> bool
    where
        Token<'src>: PartialEq,
    {
        matches!(val, Token::Keyword(kwd) if kwd == self)
    }

    fn to_maybe_ref<'b>(item: Self::Item<'b>) -> chumsky::util::MaybeRef<'src, Token<'src>>
    where
        'me: 'b,
    {
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

type LexExtra<'src> = chumsky::extra::Err<Rich<'src, char, SimpleSpan>>;

pub fn lexer<'src>() -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, LexExtra<'src>> {
    recursive(|token| {
        let dollar_ident = just('$')
            .ignore_then(text::unicode::ident())
            .map(Token::DollarIdent);

        let ident = text::unicode::ident().map(|ident| match ident {
            "fn" => Token::Keyword(Keyword::Function),
            "let" => Token::Keyword(Keyword::Let),
            "if" => Token::Keyword(Keyword::If),
            "else" => Token::Keyword(Keyword::Else),
            "true" => Token::Keyword(Keyword::True),
            "false" => Token::Keyword(Keyword::False),
            other => Token::Ident(other),
        });

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
        .map(|(op, lifts)| Token::Op { op, lifts });

        let comment = just("$.")
            .then(any().and_is(just('\n').not()).repeated())
            .padded();

        let parens = token
            .clone()
            .repeated()
            .collect()
            .delimited_by(just('('), just(')'))
            .labelled("parenthesized tokens")
            .as_context()
            .map(Token::Parens);

        let braces = token
            .repeated()
            .collect()
            .delimited_by(just('{'), just('}'))
            .labelled("braced tokens")
            .as_context()
            .map(Token::Braces);

        choice((
            //
            op,
            ident,
            dollar_ident,
            number_literal().map(Token::Number),
            parens,
            braces,
        ))
        .spanned()
        .padded_by(comment.repeated())
        .padded()
        .recover_with(skip_then_retry_until(any().ignored(), end()))
    })
    .repeated()
    .collect()
}

fn number_literal<'src>() -> impl Parser<'src, &'src str, Number, LexExtra<'src>> + Clone {
    let radix_prefix = just('0')
        .repeated()
        .ignore_then(text::digits(10).to_slice())
        .then_ignore(just('x'))
        .validate(|s: &str, e, emitter| {
            let radix = match s.parse::<u32>() {
                Ok(radix) => radix,
                Err(err) => {
                    let msg = format!("invalid radix (failed to parse it as an int: {err:?})");
                    emitter.emit(Rich::custom(e.span(), msg));
                    10
                }
            };
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
    let n_n = move |radix| {
        digit_n(radix)
            .repeated()
            .at_least(1)
            .fold(0u64, move |acc, c| match c {
                None => acc,
                Some(digit) => acc * u64::from(radix) + u64::from(digit),
            })
    };

    let kind_width = text::int(10).try_map(|s: &str, span| {
        s.parse()
            .map_err(|err| Rich::custom(span, format!("invalid width: {err:?}")))
    });
    let kind_suffix = choice((
        just('u').ignore_then(kind_width).map(NumberKind::Unsigned),
        just('i').ignore_then(kind_width).map(NumberKind::Signed),
        just('r').ignore_then(kind_width).map(NumberKind::Float),
    ));

    custom(move |inp| {
        // optionally parse a radix prefix, or 10 if unspecified
        let radix_whole = inp.parse(radix_prefix.or(empty().to(10)))?;
        let whole = inp.parse(n_n(radix_whole))?;

        let dot = inp.parse(just('.').to(true).or(empty().to(false)))?;

        // fractional part, iif there was a dot
        let frac = dot
            .then(|| {
                // optionally parse a radix prefix, or the current radix if unspecified
                let radix_frac = inp.parse(radix_prefix.or(empty().to(radix_whole)))?;
                inp.parse(n_n(radix_frac))
            })
            .transpose()?;

        let kind = inp.parse(kind_suffix.or_not())?;

        Ok(Number { whole, frac, kind })
    })
}
