pub use crate::generated::iil::flat::{OpExpr, OpExprUntyped, OpVoid};
use crate::{
    iil::{self, block},
    spv::{self, operand_kind as ok},
    types,
    util::impl_conversion_enum_variant,
};

pub trait IilOpExpr {
    fn into_spv_expr<
        MapRefs: Fn(block::BlockLocalRef) -> ok::IdRef,
        MapTypes: Fn(types::Type) -> ok::IdResultType,
    >(
        self,
        map_refs: MapRefs,
        map_types: MapTypes,
    ) -> spv::OpRetTyped;

    fn ret_type(&self) -> &types::Type;
}

pub trait IilOpVoid {
    fn into_spv_void<MapRefs: Fn(block::BlockLocalRef) -> ok::IdRef>(
        self,
        map_refs: MapRefs,
    ) -> spv::OpVoid;
}

pub trait IilOpExprUntyped {
    fn into_spv_retuntyped<MapRefs: Fn(block::BlockLocalRef) -> ok::IdRef>(
        self,
        map_refs: MapRefs,
    ) -> spv::OpRetUntyped;
}

#[derive(Debug)]
pub enum OpAnyValued {
    Typed(OpExpr),
    Untyped(OpExprUntyped),
}
impl_conversion_enum_variant!(OpAnyValued::{Typed(OpExpr), Untyped(OpExprUntyped)});

impl block::Renumberable for OpAnyValued {
    fn renumber(&mut self, from: block::BlockLocalRef, to: block::BlockLocalRef) {
        match self {
            OpAnyValued::Typed(o) => o.renumber(from, to),
            OpAnyValued::Untyped(o) => o.renumber(from, to),
        }
    }
}

pub mod instruction {
    pub use crate::generated::iil::flat::instruction::*;
    use crate::{generated::spv, iil::block, spv::operand_kind as ok, types};

    #[derive(Debug)]
    pub struct OpFunction {
        pub ret_type: types::Type,
        pub control: enumset::EnumSet<ok::FunctionControl>,
        pub r#type: types::Type,
    }

    impl block::Renumberable for OpFunction {
        fn renumber(&mut self, from: block::BlockLocalRef, to: block::BlockLocalRef) {}
    }

    impl super::IilOpExpr for OpFunction {
        fn into_spv_expr<
            MapRefs: Fn(block::BlockLocalRef) -> ok::IdRef,
            MapTypes: Fn(types::Type) -> ok::IdResultType,
        >(
            self,
            map_refs: MapRefs,
            map_types: MapTypes,
        ) -> crate::spv::OpRetTyped {
            spv::instruction::OpFunction {
                ret_type: map_types(self.ret_type),
                op0: self.control,
                op1: ok::IdRef(map_types(self.r#type).0),
            }
            .into()
        }

        fn ret_type(&self) -> &types::Type { &self.ret_type }
    }
}
