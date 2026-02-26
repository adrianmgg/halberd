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
    let atom = choice((
        just(Keyword::True).to(Expr::LiteralBool(true)),
        just(Keyword::False).to(Expr::LiteralBool(false)),
    ));
    atom.repeated().collect()
}
