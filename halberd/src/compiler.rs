use crate::{
    ast::{self, Sidecarred, Sidecars},
    scope::{self, ScopeId},
    types::{self, prelude::*},
};

mod iilifying_phase;
mod scoping_phase;
mod sidecars;
mod typing_phase;

pub(crate) use sidecars::ExprSidecar;
use sidecars::{ExprSidecarS, ExprSidecarT};

/// nothing
pub(crate) struct PhaseInitial;
impl Sidecars for PhaseInitial {
    type Expr = ExprSidecar<(), ()>;
    type Func = ();
    type ScopeItem = ();
}
/// some scopes
pub(crate) struct PhasePartiallyScoped;
impl Sidecars for PhasePartiallyScoped {
    type Expr = ExprSidecar<Option<ScopeId>, ()>;
    type Func = Option<ScopeId>;
    type ScopeItem = ();
}
/// just scope
pub(crate) struct PhaseFullyScoped;
impl Sidecars for PhaseFullyScoped {
    type Expr = ExprSidecar<ScopeId, ()>;
    type Func = ScopeId;
    type ScopeItem = ();
}
/// scope, and some types
pub(crate) struct PhasePartiallyTyped;
impl Sidecars for PhasePartiallyTyped {
    type Expr = ExprSidecar<ScopeId, Option<types::Type>>;
    type Func = ScopeId;
    type ScopeItem = NamespaceItemPartiallyTyped;
}
/// scope and fully typed
pub(crate) struct PhaseFullyTyped;
impl Sidecars for PhaseFullyTyped {
    type Expr = ExprSidecar<ScopeId, types::Type>;
    type Func = ScopeId;
    type ScopeItem = NamespaceItemFullyTyped;
}

pub(crate) struct PhaseIILGeneration;
impl Sidecars for PhaseIILGeneration {
    type Expr = ExprSidecar<ScopeId, types::Type>;
    type Func = ScopeId;
    type ScopeItem = NamespaceItemIILGeneration;
}

#[derive(Debug, Clone, Default)]
pub(crate) struct NamespaceItemPartiallyTyped {
    pub(crate) r#type: Option<types::Type>,
}

#[derive(Debug, Clone)]
pub(crate) struct NamespaceItemFullyTyped {
    pub(crate) r#type: types::Type,
}

#[derive(Debug, Clone)]
pub(crate) struct NamespaceItemIILGeneration {
    pub(crate) r#type: types::Type,
    pub(crate) block_ref: Option<crate::iil::block::BlockLocalRef>,
}

impl From<NamespaceItemFullyTyped> for NamespaceItemIILGeneration {
    fn from(value: NamespaceItemFullyTyped) -> Self {
        NamespaceItemIILGeneration { r#type: value.r#type, block_ref: None }
    }
}

pub fn compile(
    file: ast::File<'_, PhaseInitial>,
) -> Result<
    (
        ast::File<'_, PhaseFullyTyped>,
        scope::Universe<NamespaceItemFullyTyped>,
    ),
    Vec<ariadne::Report<'_>>,
> {
    let mut universe = scope::Universe::new();

    let (file, universe) = scoping_phase::populate_scopes(file, universe)?;
    let (file, universe) = typing_phase::populate_types(file, universe)?;

    Ok((file, universe))
}

pub fn foobar(
    file: ast::File<'_, PhaseFullyTyped>,
    universe: scope::Universe<NamespaceItemFullyTyped>,
) {
    let file: ast::File<'_, PhaseIILGeneration> = file
        .map_sidecars(&mut ast::SidecarFns { expr: &mut |_, car| car, func: &mut |_, car| car });
    let mut universe: scope::Universe<<PhaseIILGeneration as Sidecars>::ScopeItem> =
        universe.map(Into::into);
    iilifying_phase::process_file(file, &mut universe);
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
