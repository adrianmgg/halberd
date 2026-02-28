pub(crate) mod ast;
pub(crate) mod lexer;
pub(crate) mod parser;

use chumsky::Parser;

fn main() {
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let _ = dbg!(lexer::lexer().parse(&line).into_result());
    }
}
