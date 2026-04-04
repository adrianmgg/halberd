use crate::{
    ast::{self, NoSidecars, Sidecarred, Sidecars},
    types,
};

struct PartiallyTyped;
#[derive(Debug, PartialEq, Clone)]
struct PartiallyTypedExprData {
    r#type: Option<types::Type>,
}
impl Sidecars for PartiallyTyped {
    type Expr = PartiallyTypedExprData;
}

fn foo<'a>(e: ast::Expr<'a, NoSidecars>) -> ast::Expr<'a, PartiallyTyped> {
    // we can trivially add a type already for anything whose type is definitive from just the
    // parsed ast, everything else we will need to do more work to figure out the type later

    e.map_sidecars(|data, _| {
        let r#type = match data {
            ast::ExprData::LiteralInt(i) => Some(i.r#type.into()),
            ast::ExprData::LiteralFloat(f) => Some(f.r#type.into()),
            ast::ExprData::LiteralBool(_) => Some(types::Type::Bool),
            ast::ExprData::InfixOp(_, _, _) => None,
            ast::ExprData::Var(_) => None,
            ast::ExprData::Declaration { .. } => None,
            ast::ExprData::Block(_) => None,
        };
        PartiallyTypedExprData { r#type }
    })
}
