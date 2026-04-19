pub use crate::generated::iil::flat::{OpExpr, OpVoid};
use crate::{
    iil::{self, block},
    spv::{self, operand_kind as ok},
    types,
};

pub trait IntoSPVExpr {
    fn into_spv_expr<MapRefs: Fn(block::BlockLocalRef) -> ok::IdRef, MapTypes: Fn(types::Type)>(
        self,
        ret_id: ok::IdResult,
        map_types: MapTypes,
        map_refs: MapRefs,
    ) -> dyn spv::Instruction;
}

pub trait IntoSPVVoid {
    fn into_spv_void<MapRefs: Fn(block::BlockLocalRef) -> ok::IdRef, MapTypes: Fn(types::Type)>(
        self,
        map_types: MapTypes,
        map_refs: MapRefs,
    ) -> dyn spv::Instruction;
}

pub mod instruction {
    pub use crate::generated::iil::flat::instruction::*;
}
