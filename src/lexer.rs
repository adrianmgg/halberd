use chumsky::prelude::*;

// TODO: design decision - do we flatten these all out like e.g.
//         Let, If, Else, Ident(&str), OpAdd, OpSubtract, ...
//       or do we do sub-enums like
//         Keyword(Keyword), Ident(&str), Op(Op)

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Token<'src> {
    Keyword(Keyword),
    DollarIdent(&'src str),
    Ident(&'src str),
    Op { op: Op, lifts: usize },
    Equals,
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

// TODO see how the chumsky mini_ml example does its lexer, maybe do it that way instead?
pub fn lexer<'src>() -> impl Parser<
    'src,
    &'src str,
    Vec<Spanned<Token<'src>>>,
    chumsky::extra::Err<Rich<'src, char, SimpleSpan>>,
> {
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

    let token = choice((op, ident, dollar_ident));

    let comment = just("$.")
        .then(any().and_is(just('\n').not()).repeated())
        .padded();

    token
        .spanned()
        .padded_by(comment.repeated())
        .padded()
        .recover_with(skip_then_retry_until(any().ignored(), end()))
        .repeated()
        .collect()
}
