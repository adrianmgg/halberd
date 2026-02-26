use chumsky::prelude::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Token<'src> {
    Keyword(Keyword),
    DollarIdent(&'src str),
    Ident(&'src str),
    Op { op: Op, lifts: usize },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Keyword {
    Function,
    Let,
    If,
    Else,
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
