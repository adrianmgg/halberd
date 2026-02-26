use chumsky::prelude::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Token<'src> {
    Fn,
    DollarIdent(&'src str),
    Ident(&'src str),
}

// largely based on chumsky's "nano_rust" example
// @ https://github.com/zesterer/chumsky/blob/main/examples/nano_rust.rs
pub fn lexer<'src>() -> impl Parser<
    'src,
    &'src str,
    Vec<Spanned<Token<'src>>>,
    chumsky::extra::Err<Rich<'src, char, SimpleSpan>>,
> {
    let ident = text::unicode::ident().map(|ident| match ident {
        other => Token::Ident(other),
    });

    let token = ident;

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
