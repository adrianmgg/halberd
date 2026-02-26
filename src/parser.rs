use chumsky::input::ValueInput;
use chumsky::{pratt::*, prelude::*};

use crate::ast::{self, Expr};
use crate::lexer::{self, Keyword, Token};

type Err<'src> = chumsky::extra::Err<Rich<'src, Token<'src>, SimpleSpan>>;

fn expr_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<Expr<'src>>, Err<'src>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
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
