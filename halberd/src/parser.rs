use std::borrow::Cow;

use chumsky::input::MappedInput;
use chumsky::{Parser as _, extra, pratt::*, prelude::*};
use num_rational::BigRational;

use crate::ast::{self, Expr};
use crate::lexer::{self, Keyword, Symbol, Token};
use crate::types;

type ParserInput<'tokens, 'src> =
    MappedInput<'tokens, Token<'src>, SimpleSpan, &'tokens [Spanned<Token<'src>>]>;
type ParserErr<'tokens, 'src> = extra::Err<Rich<'tokens, Token<'src>, SimpleSpan>>;

// FIXME should rename this from `Parser` and let that be `chumsky::Parser`
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
    select_ref! { Token::Parens(ts) = e => ts.split_spanned(e.span()) }.labelled("parenthesized")
}

fn braces<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ParserInput<'tokens, 'src>> {
    select_ref! { Token::Braces(ts) = e => ts.split_spanned(e.span()) }.labelled("braced")
}

fn r#type<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, Spanned<types::Type>> {
    select! { Token::Type(t) => t }.spanned()
}

pub fn function<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ast::Function<'src>> {
    let function_arg = ident()
        .then_ignore(just(Symbol::Colon).labelled("argument name"))
        .then(r#type().labelled("argument type"))
        .map(|(name, r#type)| ast::FunctionArg { name, r#type });
    let function_args = function_arg
        .separated_by(just(Symbol::Comma))
        .collect()
        .nested_in(parens());

    let function_body = expr_parser().nested_in(parens());

    dollar_ident("fn")
        .ignore_then(ident())
        .then(function_args)
        .then_ignore(just(Symbol::Colon))
        .then(r#type())
        .then_ignore(just(Symbol::Equals))
        .then(function_body)
        .map(|(((name, args), return_type), body)| ast::Function {
            name,
            return_type,
            args,
            body,
        })
        .boxed()
}

pub fn file<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, ()> {
    // FIXME implement fully
    function().repeated()
}

pub fn expr_parser<'tokens, 'src: 'tokens>() -> impl Parser<'tokens, 'src, Expr<'src>> {
    recursive(|expr| {
        let ident = select_ref! { Token::Ident(x) => *x };
        let ident_spanned = ident.spanned();

        let expr_boxed = expr.clone().map(Box::new);

        let atom_number = select! { Token::Number(n) => n }.validate(
            |lexer::Number { value, kind }, e, emitter| match kind {
                None => {
                    emitter.emit(Rich::custom(
                        e.span(),
                        "for now, all number literals must have explicit types",
                    ));
                    ast::Expr::LiteralInt(
                        ast::LiteralInt {
                            r#type: types::Integer::Signed(64),
                            value: 1.into(),
                        }
                        .with_span(e.span()),
                    )
                }
                Some(types::NumberKind::Float(r#type)) => ast::Expr::LiteralFloat(
                    ast::LiteralFloat {
                        r#type,
                        value: match value {
                            lexer::NumberValue::Float(v) => v,
                            lexer::NumberValue::Int(v) => BigRational::new(v, 1.into()),
                        },
                    }
                    .with_span(e.span()),
                ),
                Some(types::NumberKind::Integer(r#type)) => ast::Expr::LiteralInt(
                    ast::LiteralInt {
                        r#type,
                        value: match value {
                            lexer::NumberValue::Int(v) => v,
                            lexer::NumberValue::Float(_) => {
                                emitter.emit(Rich::custom(
                                    e.span(),
                                    "number with integer type can't have decimal place",
                                ));
                                1.into()
                            }
                        },
                    }
                    .with_span(e.span()),
                ),
            },
        );

        let atom = choice((
            // true
            just(Keyword::True)
                .to(true)
                .spanned()
                .map(Expr::LiteralBool),
            // false
            just(Keyword::False)
                .to(false)
                .spanned()
                .map(Expr::LiteralBool),
            // foo
            ident.spanned().map(Expr::Var),
            // let name = ...
            just(Keyword::Let)
                .ignore_then(ident_spanned)
                .then_ignore(just(Symbol::Equals))
                .then(expr_boxed.clone())
                .map(|(name, value)| Expr::Declaration { name, value }),
            // numbers
            atom_number,
        ))
        .boxed();

        let block = expr
            .clone()
            .separated_by(just(Symbol::Semicolon))
            .collect()
            .then_ignore(just(Symbol::Semicolon))
            .then(expr_boxed.clone().or_not())
            .nested_in(select_ref! { Token::Parens(ts) = e => ts.split_spanned(e.span()) })
            .map(|(exprs, last)| ast::Block { exprs, last })
            .spanned()
            .map(Expr::Block)
            .boxed();

        // let op = |op, l| select! { Token::Op{ op: o, lifts: 0 } if o == op => () };
        // let ops = vec![infix(left(5), op(Op::Add), |l, _, r, _| {
        //     Expr::InfixOp(Box::new(l), ast::InfixOp::Add, Box::new(r))
        // })];

        choice((
            // <- load-bearing "please don't format this down to one line" comment
            atom, block,
        ))
        // .pratt(ops)
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use rstest::rstest;

    fn tokens_to_parser_input<'tokens, 'src: 'tokens>(
        src: &'src str,
        tokens: &'tokens [Spanned<Token<'src>>],
    ) -> ParserInput<'tokens, 'src> {
        tokens[..].split_spanned((0..src.len()).into())
    }

    #[rstest]
    #[case::int_literal("1u32")]
    #[case::bool_literal_true("true")]
    #[case::bool_literal_false("false")]
    #[case::variable_reference("foo")]
    fn test_expr_parses(#[case] src: &'_ str) {
        let tokens = crate::lexer::lexer()
            .parse(src)
            .into_result()
            .expect("input should lex successfully");
        let input = tokens_to_parser_input(src, &tokens[..]);
        let _ = expr_parser()
            .parse(input)
            .into_result()
            .expect("input should parse successfully");
    }

    #[rstest]
    #[case("$fn foo(): u32 = (1u32)")]
    #[case("$fn foo(x: u32, y: i32, z: r32): u32 = (1u32)")]
    fn function_parses(#[case] src: &'_ str) {
        let tokens = crate::lexer::lexer()
            .parse(src)
            .into_result()
            .expect("input should lex successfully");
        let input = tokens_to_parser_input(src, &tokens[..]);
        let _ = function()
            .parse(input)
            .into_result()
            .expect("input should parse successfully");
    }

    #[rstest]
    fn file_parses(#[files("testresources/valid/**/*.hbd")] path: PathBuf) {
        let src = std::fs::read_to_string(&path).unwrap();
        let tokens = crate::lexer::lexer()
            .parse(&src)
            .into_result()
            .expect("input should lex successfully");
        let input = tokens_to_parser_input(&src, &tokens[..]);
        let _ = file()
            .parse(input)
            .into_result()
            .expect("input should parse successfully");
    }
}
