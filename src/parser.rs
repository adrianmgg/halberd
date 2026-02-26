use chumsky::input::{BorrowInput, MappedInput, ValueInput};
use chumsky::{pratt::*, prelude::*};

use crate::ast::{self, Expr};
use crate::lexer::{self, Keyword, Token};

type Err<'src> = chumsky::extra::Err<Rich<'src, Token<'src>, SimpleSpan>>;

fn parser<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    MappedInput<'tokens, Token<'src>, SimpleSpan, &'tokens [Spanned<Token<'src>>]>,
    Spanned<Expr<'src>>,
    extra::Err<Rich<'tokens, Token<'src>>>,
> {
    recursive(|expr| {
        let ident = select_ref! { Token::Ident(x) => *x };
        let atom = choice((
            // select_ref! { Token::Num(x) => Expr::Num(*x) },
            just(Keyword::True.into()).to(Expr::LiteralBool(true)),
            just(Keyword::False.into()).to(Expr::LiteralBool(false)),
            ident.map(Expr::Var),
            just(Keyword::Let.into())
                .ignore_then(ident.spanned())
                .then_ignore(just(Token::Equals))
                .then(expr.clone())
                .map(|(name, value)| Expr::Declaration { name, value }),
        ));

        choice((
            atom.spanned(),
            // TODO temp
            expr,
            // expr.nested_in(select_ref! { Token::Parens(ts) = e => ts.split_spanned(e.span()) }),
        ))
        .pratt((
            infix(
                left(10),
                just(Token::Op {
                    op: lexer::Op::Multiply,
                    lifts: 0,
                }),
                |x, _, y, e| {
                    Expr::InfixOp(Box::new(x), ast::InfixOp::Multiply, Box::new(y))
                        .with_span(e.span())
                },
            ),
            infix(
                left(10),
                just(Token::Op {
                    op: lexer::Op::Add,
                    lifts: 0,
                }),
                |x, _, y, e| {
                    Expr::InfixOp(Box::new(x), ast::InfixOp::Add, Box::new(y)).with_span(e.span())
                },
            ),
        ))
        // .labelled("expression")
        // .as_context()
    })
}

/*
fn expr_parser<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    MappedInput<'tokens, Token<'src>, SimpleSpan, &'tokens [Spanned<Token<'src>>]>,
    Spanned<Expr<'src>>,
    Err<'src>,
> + Clone {
    recursive(|expr| {
        let ident = select_ref! { Token::Ident(x) => *x };
        let atom = choice((
            ident.map(Expr::Var),
            just(Token::Keyword(Keyword::True)).to(Expr::LiteralBool(true)),
            just(Token::Keyword(Keyword::False)).to(Expr::LiteralBool(true)),
        ));
        atom.spanned()
            .pratt((infix(
                left(1),
                just(Token::Op {
                    op: lexer::Op::Multiply,
                    lifts: 0,
                }),
                |x, _, y, e| {
                    Expr::InfixOp(Box::new(x), ast::InfixOp::Multiply, Box::new(y))
                        .with_span(e.span())
                },
            ),))
            .labelled("expression")
            .as_context()
    })
}
*/
