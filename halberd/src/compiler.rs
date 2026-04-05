use crate::{
    ast::{self, NoSidecars, SidecarFns, Sidecarred, Sidecars},
    types,
};

pub(crate) struct PartiallyTyped;
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct PartiallyTypedExprData {
    pub(crate) r#type: Option<types::Type>,
}
impl Sidecars for PartiallyTyped {
    type Expr = PartiallyTypedExprData;
}

pub fn foo<'a>(e: ast::Expr<'a, NoSidecars>) -> ast::Expr<'a, PartiallyTyped> {
    // we can trivially add a type already for anything whose type is definitive from just the
    // parsed ast, everything else we will need to do more work to figure out the type later

    let mut e: ast::Expr<'a, PartiallyTyped> = e.map_sidecars(&SidecarFns {
        expr: |data: &ast::ExprData<'a>, _| {
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
        },
    });

    // e.iteratively_modify_sidecars(&SidecarFns {
    //     expr: |data: &ast::ExprData<'a, PartiallyTyped>, sidecar: &mut PartiallyTypedExprData| {
    //         match sidecar.r#type {
    //             Some(_) => false,
    //             None => match data {
    //                 ast::ExprData::LiteralInt(spanned) => todo!(),
    //                 ast::ExprData::LiteralFloat(spanned) => todo!(),
    //                 ast::ExprData::LiteralBool(spanned) => todo!(),
    //                 ast::ExprData::InfixOp(expr, spanned, expr1) => todo!(),
    //                 ast::ExprData::Var(spanned) => todo!(),
    //                 ast::ExprData::Declaration { name, value } => todo!(),
    //                 ast::ExprData::Block(spanned) => todo!(),
    //             },
    //         }
    //     },
    // });
    e
}
