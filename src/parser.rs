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
    Expr<'src>,
    extra::Err<Rich<'tokens, Token<'src>, SimpleSpan>>,
>
where
    // I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
    I: BorrowInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    recursive(|expr| {
        // FIXME select_ref if end up using BorrowInput -- remove the other one when done
        let ident = select_ref! { Token::Ident(x) => *x };
        // let ident = select! { Token::Ident(x) => x };

        let expr_boxed = expr.map(Box::new);

        let atom = choice((
            // true
            just(Keyword::True).to(Expr::LiteralBool(true)),
            // false
            just(Keyword::False).to(Expr::LiteralBool(false)),
            // foo
            ident.map(Expr::Var),
            // let name = ...
            just(Keyword::Let)
                .ignore_then(ident)
                .then_ignore(just(Token::Equals))
                .then(expr_boxed.clone())
                .map(|(name, value)| Expr::Declaration { name, value }),
        ));

        let fn_args = just(Keyword::True).to(());
        let fn_def = just(Keyword::Function).ignore_then(fn_args.nested_in());

        atom.boxed()
    })
}
