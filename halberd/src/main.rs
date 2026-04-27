#![feature(trait_alias)]
#![feature(try_blocks)]
#![feature(macro_metavar_expr_concat)]
// #![feature(min_specialization)]
#![feature(iterator_try_collect)]
#![feature(iter_intersperse)]
// FIXME should probably turn this back on once we reach a 1.0
#![allow(unused)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::too_many_lines,
    clippy::inline_always,
    clippy::unreadable_literal,
    clippy::wildcard_imports,
    clippy::default_trait_access
)]
#![deny(clippy::ignored_unit_patterns, clippy::semicolon_if_nothing_returned)]

pub(crate) mod ast;
pub(crate) mod compiler;
pub(crate) mod generated;
pub(crate) mod iil;
pub(crate) mod lexer;
pub(crate) mod parser;
pub(crate) mod scope;
pub(crate) mod spv;
pub(crate) mod tex;
pub(crate) mod types;
pub(crate) mod util;

use ariadne::{Label, Report, ReportKind};
use chumsky::{Parser, input::Input};

fn main() {
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let src = ariadne::Source::from(&line);
        // FIXME should eventually really be using `.into_output_errors` instead of `.into_result`
        let tokens = match lexer::lexer().parse(&line).into_result() {
            Ok(tokens) => tokens,
            Err(errs) => {
                for err in errs {
                    parser_error_to_report(err).eprint(&src);
                }
                continue;
            }
        };
        println!("{}", tex::Tex(&(tokens.as_slice(), line.as_ref())));
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
        println!("{}", tex::Tex(&file));
        println!(
            "================================================================================"
        );
        match compiler::compile(file) {
            Ok((file, universe)) => {
                // dbg!(&file, &universe);
                println!(
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
