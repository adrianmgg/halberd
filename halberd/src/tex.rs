use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

use chumsky::span::Spanned;

use crate::{
    ast,
    compiler::{
        self,
        sidecars::{ExprSidecarS as _, ExprSidecarT as _},
    },
    lexer, scope,
};

pub(crate) trait DebugTex {
    fn fmt_tex(&self, f: &mut Formatter<'_>) -> fmt::Result;
}

pub(crate) struct Tex<'a, T: DebugTex>(pub(crate) &'a T);

impl<T: DebugTex> Display for Tex<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { self.0.fmt_tex(f) }
}

fn indent(f: &mut Formatter<'_>, n: usize) -> fmt::Result {
    for _ in 0..n {
        f.write_str("    ")?;
    }
    Ok(())
}

fn forest_dump_ast_graph<S>(
    expr: &ast::Expr<'_, S>,
    f: &mut Formatter<'_>,
    tabs: usize,
) -> fmt::Result
where
    S: ast::Sidecars,
    S::Expr: DebugTexSidecar,
{
    f.write_str("[")?;

    f.write_str("{")?;
    let should_write_sidecar = expr.sidecar.has_fmt_sidecar_tex();
    if should_write_sidecar {
        f.write_str(r"\(\overset{\text{")?;
        expr.sidecar.fmt_sidecar_tex(f)?;
        f.write_str(r"}}{\text{")?;
    }
    match &expr.data {
        ast::ExprData::LiteralInt(spanned) => write!(f, "{}", spanned.inner.value)?,
        ast::ExprData::LiteralFloat(spanned) => write!(f, "{}", spanned.inner.value)?,
        ast::ExprData::LiteralBool(spanned) => write!(f, "\\(\\texttt{{{}}}\\)", spanned.inner)?,
        ast::ExprData::InfixOp(lhs, op, rhs) => write!(f, r"{:?}", op.inner)?,
        ast::ExprData::Var(spanned) => write!(f, r"\texttt{{{}}}", spanned.inner)?,
        ast::ExprData::Declaration { name, r#type, value } =>
            write!(f, r"Declare~\texttt{{{}}}", name.inner)?,
        ast::ExprData::Block(spanned) => f.write_str(r"Block")?,
    }
    if should_write_sidecar {
        f.write_str(r"}}\)")?;
    }
    f.write_str("} ")?;

    match &expr.data {
        ast::ExprData::LiteralInt(spanned) => {}
        ast::ExprData::LiteralFloat(spanned) => {}
        ast::ExprData::LiteralBool(spanned) => {}
        ast::ExprData::InfixOp(lhs, op, rhs) => {
            f.write_str("\n")?;
            indent(f, tabs + 1)?;
            forest_dump_ast_graph(lhs, f, tabs + 1)?;
            f.write_str("\n")?;
            indent(f, tabs + 1)?;
            forest_dump_ast_graph(rhs, f, tabs + 1)?;
            f.write_str("\n")?;
            indent(f, tabs)?;
        }
        ast::ExprData::Var(spanned) => {}
        ast::ExprData::Declaration { name, r#type, value } => {
            f.write_str("\n")?;
            indent(f, tabs + 1)?;
            forest_dump_ast_graph(value, f, tabs + 1)?;
            f.write_str("\n")?;
            indent(f, tabs)?;
        }
        ast::ExprData::Block(spanned) => {
            f.write_str(r" [{\textit{exprs}} ")?;
            for child in &spanned.inner.exprs {
                f.write_str("\n")?;
                indent(f, tabs + 1)?;
                forest_dump_ast_graph(child, f, tabs + 1)?;
            }
            f.write_str(r"] [{\textit{terminal}} ")?;
            if let Some(terminal) = &spanned.inner.last {
                f.write_str("\n")?;
                indent(f, tabs + 1)?;
                forest_dump_ast_graph(terminal, f, tabs + 1)?;
            }
            f.write_str("] \n")?;
            indent(f, tabs)?;
        }
    }
    f.write_str("]")
}

fn forest_dump_function<S>(
    function: &ast::Function<'_, S>,
    f: &mut Formatter<'_>,
    tabs: usize,
) -> fmt::Result
where
    S: ast::Sidecars,
    S::Expr: DebugTexSidecar,
{
    writeln!(
        f,
        r"[{{function~\texttt{{{}}}}}",
        function.data.name.as_ref()
    );
    forest_dump_ast_graph(&function.data.body, f, tabs + 1)?;
    f.write_str("]")
}

impl<S> DebugTex for ast::File<'_, S>
where
    S: ast::Sidecars,
    S::Expr: DebugTexSidecar,
{
    fn fmt_tex(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("\\begin{forest}\n")?;
        for (name, functions) in &self.functions {
            for function in functions {
                indent(f, 1)?;
                forest_dump_function(function, f, 1)?;
            }
        }
        f.write_str("\n\\end{forest}\n")
    }
}

fn escape_for_tex(txt: &'_ str) -> Cow<'_, str> {
    const TO_ESCAPE: &[char] = &['\\', '$', '{', '}'];
    let mut txt = Cow::Borrowed(txt);
    for c in TO_ESCAPE {
        if txt.contains(*c) {
            txt = Cow::Owned(txt.replace(*c, &format!("\\{c}")));
        }
    }
    txt
}

pub(crate) trait DebugTexSidecar {
    fn has_fmt_sidecar_tex(&self) -> bool;
    fn fmt_sidecar_tex(&self, f: &mut Formatter<'_>) -> fmt::Result { Ok(()) }
}

pub(crate) trait DebugTexNamespaceSidecar {
    fn namespace_item_columns() -> &'static [&'static str];
    fn namespace_item_entries(&self) -> impl Iterator<Item = String>;
}

impl DebugTex for (&[Spanned<lexer::Token<'_>>], &'_ str) {
    fn fmt_tex(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fn dump_toks(
            toks: &[Spanned<lexer::Token<'_>>],
            src: &str,
            f: &mut Formatter<'_>,
        ) -> fmt::Result {
            for token in toks {
                let kind = match &token.inner {
                    lexer::Token::Keyword(..) => "Keyword",
                    lexer::Token::Symbol(..) => "Symbol",
                    lexer::Token::DollarIdent(..) => "DollarIdent",
                    lexer::Token::Ident(..) => "Ident",
                    lexer::Token::Op { .. } => "Op",
                    lexer::Token::Parens(..) => "Parens",
                    lexer::Token::Braces(..) => "Braces",
                    lexer::Token::Number(..) => "Number",
                    lexer::Token::Type(..) => "Type",
                };
                let txt = &src[token.span.into_range()];
                let contents = match &token.inner {
                    lexer::Token::Parens(t) => Some((t, "(", ")")),
                    lexer::Token::Braces(t) => Some((t, "{", "}")),
                    _ => None,
                };
                match contents {
                    Some((contents, open, close)) => {
                        write!(
                            f,
                            r"\LexerTokenTreeOpen{{{open}}}{{{kind}}}",
                            open = escape_for_tex(open),
                        )?;
                        dump_toks(contents, src, f)?;
                        write!(
                            f,
                            r"\LexerTokenTreeClose{{{close}}}{{{kind}}}",
                            close = escape_for_tex(close),
                        )?;
                    }
                    None => {
                        write!(
                            f,
                            r"\LexerToken{{{txt}}}{{{kind}}}",
                            txt = escape_for_tex(txt)
                        )?;
                    }
                }
            }
            Ok(())
        }

        let (toks, src) = self;

        f.write_str("\\(  % lexer output\n")?;
        dump_toks(toks, src, f)?;
        f.write_str("\n)")
    }
}

impl DebugTexNamespaceSidecar for () {
    fn namespace_item_columns() -> &'static [&'static str] { &[] }

    fn namespace_item_entries(&self) -> impl Iterator<Item = String> { std::iter::empty() }
}

impl DebugTexNamespaceSidecar for compiler::NamespaceItemFullyTyped {
    fn namespace_item_columns() -> &'static [&'static str] { &["type"] }

    fn namespace_item_entries(&self) -> impl Iterator<Item = String> {
        std::iter::once(format!(r"\texttt{{{}}}", self.r#type))
    }
}

impl DebugTexSidecar for () {
    fn has_fmt_sidecar_tex(&self) -> bool { false }
}

impl DebugTexSidecar for <compiler::PhaseInitial as ast::Sidecars>::Expr {
    fn has_fmt_sidecar_tex(&self) -> bool { false }
}

impl DebugTexSidecar for <compiler::PhaseFullyScoped as ast::Sidecars>::Expr {
    fn has_fmt_sidecar_tex(&self) -> bool { true }

    fn fmt_sidecar_tex(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.scope())
    }
}

impl DebugTexSidecar for <compiler::PhaseFullyTyped as ast::Sidecars>::Expr {
    fn has_fmt_sidecar_tex(&self) -> bool { true }

    fn fmt_sidecar_tex(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.r#type())
    }
}
