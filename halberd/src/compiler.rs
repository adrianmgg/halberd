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

    e.iteratively_modify_sidecars(&SidecarFns {
        expr: |data: &ast::ExprData<'a, PartiallyTyped>, sidecar: &mut PartiallyTypedExprData| {
            match sidecar.r#type {
                Some(_) => false,
                // FIXME rewrite this to handle the return bool automatically
                None => match data {
                    ast::ExprData::LiteralInt(_) => false,
                    ast::ExprData::LiteralFloat(_) => false,
                    ast::ExprData::LiteralBool(_) => false,
                    ast::ExprData::InfixOp(lhs, op, rhs) => todo!(),
                    ast::ExprData::Var(_) => false,
                    // FIXME wait. is our ast wrong here WHOOPS
                    ast::ExprData::Declaration { name: _, value } => match value.sidecar.r#type {
                        Some(r#type) => {
                            sidecar.r#type = Some(r#type);
                            true
                        }
                        None => false,
                    },
                    ast::ExprData::Block(spanned) => match &spanned.inner.last {
                        // blocks with no terminal expression get the type void
                        None => {
                            sidecar.r#type = Some(types::Type::Void);
                            true
                        }
                        // blocks with a terminal expression get that expression's type if it has one
                        Some(terminal) => match terminal.sidecar.r#type {
                            Some(r#type) => {
                                sidecar.r#type = Some(r#type);
                                true
                            }
                            None => false,
                        },
                    },
                },
            }
        },
    });

    e
}
