use std::borrow::Cow;

use chumsky::input::MappedInput;
use chumsky::{extra, pratt::*, prelude::*};

use crate::ast::{self, Expr};
use crate::lexer::{Keyword, Symbol, Token};

type ParserInput<'tokens, 'src> =
    MappedInput<'tokens, Token<'src>, SimpleSpan, &'tokens [Spanned<Token<'src>>]>;
type ParserErr<'tokens, 'src> = extra::Err<Rich<'tokens, Token<'src>, SimpleSpan>>;

pub trait Parser<'tokens, 'src: 'tokens, T> =
    chumsky::Parser<'tokens, ParserInput<'tokens, 'src>, T, ParserErr<'tokens, 'src>>;

fn ident<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, Spanned<Cow<'src, str>>> {
    select! { Token::Ident(x) => x }
        .map(Cow::Borrowed)
        .spanned()
}

fn any_dollar_ident<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, Spanned<Cow<'src, str>>>
{
    select! { Token::DollarIdent(x) => x }
        .map(Cow::Borrowed)
        .spanned()
}

fn dollar_ident<'tokens, 'src: 'tokens>(
    ident: &'static str,
) -> impl Parser<'tokens, 'src, Spanned<Cow<'src, str>>> {
    select! { Token::DollarIdent(x) if x == ident => x }
        .map(Cow::Borrowed)
        .spanned()
}

fn parens<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ParserInput<'tokens, 'src>> {
    select_ref! { Token::Parens(ts) = e => ts.split_spanned(e.span()) }
}

fn braces<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ParserInput<'tokens, 'src>> {
    select_ref! { Token::Braces(ts) = e => ts.split_spanned(e.span()) }
}

pub fn function<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Function<'src>> {
    let function_arg = ident()
        .then_ignore(just(Symbol::Colon))
        .then(todo::<_, ast::Type, _>().spanned())
        .map(|(name, r#type)| ast::FunctionArg { name, r#type });
    let function_args = function_arg
        .separated_by(just(Symbol::Comma))
        .collect()
        .nested_in(parens());

    let function_body = expr_parser();

    dollar_ident("fn")
        .ignore_then(ident())
        .then(function_args)
        .then(function_body)
        .map(|((name, args), body)| ast::Function { name, args, body })
        .boxed()
}

pub fn expr_parser<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, Spanned<Expr<'src>>> {
    recursive(|expr| {
        let ident = select_ref! { Token::Ident(x) => *x };
        let ident_spanned = ident.spanned();

        let expr_boxed = expr.clone().map(Box::new);

        let atom = choice((
            // true
            just(Keyword::True).to(Expr::LiteralBool(true)),
            // false
            just(Keyword::False).to(Expr::LiteralBool(false)),
            // foo
            ident.map(Expr::Var),
            // let name = ...
            just(Keyword::Let)
                .ignore_then(ident_spanned)
                .then_ignore(just(Symbol::Equals))
                .then(expr_boxed.clone())
                .map(|(name, value)| Expr::Declaration { name, value }),
        ))
        .boxed();

        let block = expr
            .clone()
            .separated_by(just(Symbol::Semicolon))
            .collect()
            .then_ignore(just(Symbol::Semicolon))
            .then(expr_boxed.clone().or_not())
            .nested_in(select_ref! { Token::Parens(ts) = e => ts.split_spanned(e.span()) })
            .map(|(exprs, last)| Expr::Block(ast::Block { exprs, last }))
            .boxed();

        choice((
            // <- load-bearing "please don't format this down to one line" comment
            atom, block,
        ))
        .spanned()
    })
}
