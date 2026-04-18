use crate::{
    ast::{self, Sidecarred, Sidecars},
    scope::{self, ScopeId},
    types::{self, prelude::*},
};

mod scoping_phase;
mod sidecars;
mod typing_phase;

pub(crate) use sidecars::ExprSidecar;
use sidecars::{ExprSidecarS, ExprSidecarT};

/// nothing
pub(crate) struct NoSidecars;
impl Sidecars for NoSidecars {
    type Expr = ExprSidecar<(), ()>;
    type Func = ();
}
/// some scopes
pub(crate) struct PhasePartiallyScoped;
impl Sidecars for PhasePartiallyScoped {
    type Expr = ExprSidecar<Option<ScopeId>, ()>;
    type Func = Option<ScopeId>;
}
/// just scope
pub(crate) struct PhaseFullyScoped;
impl Sidecars for PhaseFullyScoped {
    type Expr = ExprSidecar<ScopeId, ()>;
    type Func = ScopeId;
}
/// scope, and some types
pub(crate) struct PhasePartiallyTyped;
impl Sidecars for PhasePartiallyTyped {
    type Expr = ExprSidecar<ScopeId, Option<types::Type>>;
    type Func = ScopeId;
}
/// scope and fully typed
pub(crate) struct PhaseFullyTyped;
impl Sidecars for PhaseFullyTyped {
    type Expr = ExprSidecar<ScopeId, types::Type>;
    type Func = ScopeId;
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NamespaceItemNothing;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NamespaceItemPartiallyTyped {
    pub(crate) r#type: Option<types::Type>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NamespaceItemFullyTyped {
    pub(crate) r#type: types::Type,
}

pub fn compile<'a>(
    e: ast::File<'a, NoSidecars>,
) -> Result<
    (
        ast::File<'a, PhaseFullyTyped>,
        scope::Universe<NamespaceItemFullyTyped>,
    ),
    Vec<ariadne::Report<'a>>,
> {
    let mut universe = scope::Universe::new();

    let (e, universe) = scoping_phase::populate_scopes(e, universe)?;
    let (e, universe) = typing_phase::populate_types(e, universe)?;

    Ok((e, universe))
}

#[cfg(test)]
mod tests {
    use std::{assert_matches, path::PathBuf};

    use rstest::rstest;

    #[rstest]
    fn file_parses(#[files("testresources/valid/**/*.hbd")] path: PathBuf) {
        use chumsky::Parser as _;

        use crate::parser::tokens_to_parser_input;

        let src = std::fs::read_to_string(&path).unwrap();
        let tokens = crate::lexer::lexer()
            .parse(&src)
            .into_result()
            .expect("input should lex successfully");
        let input = tokens_to_parser_input(&src, &tokens[..]);
        let file = crate::parser::file()
            .parse(input)
            .into_result()
            .expect("input should parse successfully");
        assert_matches!(super::compile(file), Ok(_));
    }
}
