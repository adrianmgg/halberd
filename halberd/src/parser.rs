use std::borrow::Cow;

use chumsky::input::MappedInput;
use chumsky::{extra, pratt::*, prelude::*};

use crate::ast::{self, Expr};
use crate::lexer::{Keyword, Symbol, Token};

type ParserInput<'tokens, 'src> =
    MappedInput<'tokens, Token<'src>, SimpleSpan, &'tokens [Spanned<Token<'src>>]>;
type ParserErr<'tokens, 'src> = extra::Err<Rich<'tokens, Token<'src>, SimpleSpan>>;

fn ident<'tokens, 'src: 'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens, 'src>, Spanned<Cow<'src, str>>, ParserErr<'tokens, 'src>>
{
    select! { Token::Ident(x) => x }
        .map(Cow::Borrowed)
        .spanned()
}

fn any_dollar_ident<'tokens, 'src: 'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens, 'src>, Spanned<Cow<'src, str>>, ParserErr<'tokens, 'src>>
{
    select! { Token::DollarIdent(x) => x }
        .map(Cow::Borrowed)
        .spanned()
}

fn dollar_ident<'tokens, 'src: 'tokens>(
    ident: &'static str,
) -> impl Parser<'tokens, ParserInput<'tokens, 'src>, Spanned<Cow<'src, str>>, ParserErr<'tokens, 'src>>
{
    select! { Token::DollarIdent(x) if x == ident => x }
        .map(Cow::Borrowed)
        .spanned()
}

fn parens<'tokens, 'src: 'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens, 'src>, ParserInput<'tokens, 'src>, ParserErr<'tokens, 'src>>
{
    select_ref! { Token::Parens(ts) = e => ts.split_spanned(e.span()) }
}

fn braces<'tokens, 'src: 'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens, 'src>, ParserInput<'tokens, 'src>, ParserErr<'tokens, 'src>>
{
    select_ref! { Token::Braces(ts) = e => ts.split_spanned(e.span()) }
}

pub fn function_parser<'tokens, 'src: 'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens, 'src>, ast::Function<'src>, ParserErr<'tokens, 'src>> {
    let function_arg = ident()
        .then_ignore(just(Symbol::Colon))
        .then(todo::<_, ast::Type, _>())
        // FIXME handle spans better in here
        .map(|(name, r#type)| ast::FunctionArg {
            name: name.inner,
            r#type,
        });
    let function_args = function_arg.separated_by(just(Symbol::Comma));
    dollar_ident("fn")
        .ignore_then(function_args.nested_in(parens()))
        .then(braces());
    todo()
}

pub fn expr_parser<'tokens, 'src: 'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens, 'src>, Spanned<Expr<'src>>, ParserErr<'tokens, 'src>> {
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

        // // let fn_args = just(Keyword::True).to(());
        // let fn_def = just(Keyword::Function)
        //     .ignore_then(ident)
        //     .then(
        //         expr_boxed
        //             .nested_in(select_ref! { Token::Braces(ts) = e => ts.split_spanned(e.span()) }),
        //     )
        //     .map(|(name, body)| Expr::FunctionDeclaration { name, body });

        let block = expr
            .clone()
            .separated_by(just(Symbol::Semicolon))
            .collect()
            .then_ignore(just(Symbol::Semicolon))
            .then(expr_boxed.clone().or_not())
            .nested_in(select_ref! { Token::Parens(ts) = e => ts.split_spanned(e.span()) })
            .map(|(exprs, last)| Expr::Block { exprs, last })
            .boxed();

        choice((
            // <- load-bearing "please don't format this down to one line" comment
            atom, block,
        ))
        .spanned()
    })
}
