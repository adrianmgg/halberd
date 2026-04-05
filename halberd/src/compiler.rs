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
        expr: |_: &ast::ExprData<'a>, _: ()| PartiallyTypedExprData { r#type: None },
    });

    e.iteratively_modify_sidecars(&SidecarFns {
        expr: |data: &ast::ExprData<'a, PartiallyTyped>, sidecar: &mut PartiallyTypedExprData| {
            match sidecar.r#type {
                Some(_) => false,
                // FIXME rewrite this to handle the return bool automatically
                None => {
                    let r#type = match data {
                        ast::ExprData::LiteralInt(i) => Some(i.r#type.into()),
                        ast::ExprData::LiteralFloat(f) => Some(f.r#type.into()),
                        ast::ExprData::LiteralBool(_) => Some(types::Type::Bool),
                        ast::ExprData::InfixOp(lhs, op, rhs) => match op.inner {
                            ast::InfixOp::Add | ast::InfixOp::Subtract | ast::InfixOp::Multiply => {
                                todo!()
                            }
                            ast::InfixOp::Divide => todo!(),
                            ast::InfixOp::DotProduct => todo!(),
                            ast::InfixOp::CrossProduct => todo!(),
                            ast::InfixOp::MatrixMultiply => todo!(),
                        },
                        ast::ExprData::Var(_) => None,
                        // FIXME wait. is our ast wrong here WHOOPS
                        ast::ExprData::Declaration { name: _, value } => value.sidecar.r#type,
                        ast::ExprData::Block(spanned) => match &spanned.inner.last {
                            // blocks with no terminal expression get the type void
                            None => Some(types::Type::Void),
                            // blocks with a terminal expression get that expression's type if it has one
                            Some(terminal) => terminal.sidecar.r#type,
                        },
                    };
                    match r#type {
                        None => false,
                        Some(r#type) => {
                            sidecar.r#type = Some(r#type);
                            true
                        }
                    }
                }
            }
        },
    });

    e
}
