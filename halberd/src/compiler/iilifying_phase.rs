use super::PhaseFullyTyped;
use crate::{
    ast::{self, Sidecars},
    iil, scope,
    spv::operand_kind as ok,
    types,
};

fn foo<'a>(
    file: ast::File<'a, PhaseFullyTyped>,
    universe: scope::Universe<<PhaseFullyTyped as Sidecars>::ScopeItem>,
) -> iil::h::instruction::OpFunction {
    iil::h::instruction::OpFunction {
        control: ok::FunctionControl::None,
        op1: types::Type::Function,
    }
}
