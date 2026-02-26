use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lexer::Token;

fn expr_parser<'tokens, 'src: 'tokens, I>() -> impl Parser<
    'tokens,
    I,
    Spanned<ast::Expr<'src>>,
    chumsky::extra::Err<Rich<'src, Token<'src>, SimpleSpan>>,
> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
}
