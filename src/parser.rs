use chumsky::input::{BorrowInput, MappedInput, ValueInput};
use chumsky::{extra, pratt::*, prelude::*};

use crate::ast::{self, Expr};
use crate::lexer::{self, Keyword, Token};

type Err<'src> = chumsky::extra::Err<Rich<'src, Token<'src>, SimpleSpan>>;

pub fn parser<'tokens, 'src: 'tokens, I>() -> impl Parser<
    //
    'tokens,
    I,
    // Spanned<Expr<'src>>,
    Vec<Expr<'src>>,
    extra::Err<Rich<'tokens, Token<'src>, SimpleSpan>>,
>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    let atom = just(Token::Keyword(Keyword::True)).to(Expr::LiteralBool(true));
    atom.repeated().collect()
}
