#![feature(trait_alias)]
#![feature(try_blocks)]
#![feature(macro_metavar_expr_concat)]
// #![feature(min_specialization)]

// FIXME should probably turn this back on once we reach a 1.0
#![allow(unused)]

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
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let src = ariadne::Source::from(&line);
        // FIXME should eventually really be using `.into_output_errors` instead of `.into_result`
        let tokens = match dbg!(lexer::lexer().parse(&line).into_result()) {
            Ok(tokens) => tokens,
            Err(errs) => {
                for err in errs.into_iter() {
                    parser_error_to_report(err).eprint(&src);
                }
                continue;
            }
        };
        let parser_input = tokens[..].split_spanned((0..line.len()).into());
        let file = match parser::file().parse(parser_input).into_result() {
            Ok(file) => file,
            Err(errs) => {
                for err in errs.into_iter() {
                    parser_error_to_report(err).eprint(&src);
                }
                continue;
            }
        };
        println!(
            "================================================================================"
        );
        match compiler::compile(file) {
            Ok((file, universe)) => {
                dbg!(file, universe);
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
