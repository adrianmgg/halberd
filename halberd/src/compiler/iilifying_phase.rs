use super::PhaseFullyTyped;
use crate::{
    ast::{self, Sidecars},
    iil, scope, spv, types,
};

fn foo<'a>(
    file: ast::File<'a, PhaseFullyTyped>,
    universe: scope::Universe<<PhaseFullyTyped as Sidecars>::ScopeItem>,
) -> iil::h::instruction::OpFunction {
    iil::h::instruction::OpFunction {
        op0: spv::operand_kind::FunctionControl::None,
        op1: types::Type::Function,
    }
}
