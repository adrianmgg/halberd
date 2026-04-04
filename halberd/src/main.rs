#![feature(trait_alias)]

pub(crate) mod ast;
pub(crate) mod compiler;
pub(crate) mod generated;
pub(crate) mod iil;
pub(crate) mod lexer;
pub(crate) mod parser;
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
            let _ = dbg!(parser::function().parse(input).into_result());
        }
    }
}
