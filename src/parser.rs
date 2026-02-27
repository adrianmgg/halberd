use chumsky::input::MappedInput;
use chumsky::{extra, pratt::*, prelude::*};

use crate::ast::Expr;
use crate::lexer::{Keyword, Token};

pub fn parser<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    MappedInput<'tokens, Token<'src>, SimpleSpan, &'tokens [Spanned<Token<'src>>]>,
    Spanned<Expr<'src>>,
    extra::Err<Rich<'tokens, Token<'src>, SimpleSpan>>,
> {
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
                .then_ignore(just(Token::Equals))
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
            .separated_by(just(Token::Semicolon))
            .collect()
            .then_ignore(just(Token::Semicolon))
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
