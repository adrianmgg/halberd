pub(crate) mod ast;
pub(crate) mod generated;
pub(crate) mod lexer;
pub(crate) mod parser;
pub(crate) mod spv;
pub(crate) mod iil;

use chumsky::Parser;

fn main() {
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let _ = dbg!(lexer::lexer().parse(&line).into_result());
    }
}
