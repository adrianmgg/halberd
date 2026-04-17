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

use chumsky::{Parser, input::Input};

fn main() {
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let tokens = dbg!(lexer::lexer().parse(&line).into_result());
        if let Ok(tokens) = tokens {
            let input = tokens[..].split_spanned((0..line.len()).into());
            if let Ok(file) = dbg!(parser::file().parse(input).into_result()) {
                let expr_typed = compiler::compile(file);
                dbg!(&expr_typed);
            }
        }
    }
}
