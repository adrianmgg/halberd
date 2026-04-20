pub use crate::generated::iil::flat::{OpExpr, OpExprUntyped, OpVoid};
use crate::{
    iil::{self, block},
    spv::{self, operand_kind as ok},
    types,
};

pub trait IilOpExpr {
    fn into_spv_expr<
        MapRefs: Fn(block::BlockLocalRef) -> ok::IdRef,
        MapTypes: Fn(types::Type) -> ok::IdResultType,
    >(
        self,
        map_refs: MapRefs,
        map_types: MapTypes,
    ) -> impl spv::Instruction;

    fn ret_type(&self) -> &types::Type;
}

pub trait IilOpVoid {
    fn into_spv_void<MapRefs: Fn(block::BlockLocalRef) -> ok::IdRef>(
        self,
        map_refs: MapRefs,
    ) -> impl spv::Instruction;
}

pub mod instruction {
    pub use crate::generated::iil::flat::instruction::*;
}
