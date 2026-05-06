#![feature(trait_alias)]
#![feature(try_blocks)]
#![feature(macro_metavar_expr_concat)]
// #![feature(min_specialization)]
#![feature(iterator_try_collect)]
#![feature(int_roundings)]
// FIXME should probably turn this back on once we reach a 1.0
#![allow(unused, reason = "i'm being lazy for now")]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::too_many_lines,
    clippy::inline_always,
    clippy::unreadable_literal,
    clippy::wildcard_imports,
    clippy::default_trait_access,
    reason = "i just dont really care about these"
)]
#![deny(
    clippy::ignored_unit_patterns,
    clippy::semicolon_if_nothing_returned,
    clippy::wildcard_enum_match_arm,
    clippy::clone_on_copy,
    clippy::allow_attributes_without_reason
)]

pub(crate) mod ast;
pub(crate) mod compiler;
pub(crate) mod generated;
pub(crate) mod iil;
pub(crate) mod lexer;
pub(crate) mod parser;
pub(crate) mod scope;
pub(crate) mod spv;
pub(crate) mod types;
pub(crate) mod util;

use ariadne::{Label, Report, ReportKind};
use chumsky::{Parser, input::Input};

fn main() {
    let argv: Vec<_> = std::env::args().skip(1).collect();
    let repl_lines: Box<dyn Iterator<Item = String>> = if argv.is_empty() {
        Box::new(std::io::stdin().lines().map(|line| line.unwrap()))
    } else {
        Box::new(
            argv.into_iter()
                .map(|f| std::fs::read_to_string(f).unwrap()),
        )
    };

    for line in repl_lines {
        let src = ariadne::Source::from(&line);
        // FIXME should eventually really be using `.into_output_errors` instead of `.into_result`
        let tokens = match dbg!(lexer::lexer().parse(&line).into_result()) {
            Ok(tokens) => tokens,
            Err(errs) => {
                for err in errs {
                    parser_error_to_report(err).eprint(&src);
                }
                continue;
            }
        };
        let parser_input = tokens[..].split_spanned((0..line.len()).into());
        let file = match parser::file().parse(parser_input).into_result() {
            Ok(file) => file,
            Err(errs) => {
                for err in errs {
                    parser_error_to_report(err).eprint(&src);
                }
                continue;
            }
        };
        eprintln!(
            "================================================================================"
        );
        match compiler::compile(file) {
            Ok((file, universe)) => {
                dbg!(&file, &universe);
                eprintln!(
                    "================================================================================"
                );
                compiler::foobar(file, universe);
            }
            Err(errors) =>
                for error in errors {
                    error.eprint(&src);
                },
        }
    }
}

fn parser_error_to_report<T: std::fmt::Display>(
    e: chumsky::error::Rich<'_, T>,
) -> Report<'_, ((), std::ops::Range<usize>)> {
    Report::build(ReportKind::Error, ((), e.span().into_range()))
        .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
        .with_message(e.to_string())
        .with_label(Label::new(((), e.span().into_range())).with_message(e.reason().to_string()))
        .finish()
}
