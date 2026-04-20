use std::collections::{HashMap, HashSet};

use rstest::rstest;

use crate::{
    iil::block,
    types::{self, to_spv::TypeToSpv},
};

// FIXME no longer spvifying... move.

pub(crate) fn types_to_asm(
    mut types_to_build: HashSet<types::Type>,
    blockctx: &mut block::Ctx,
) -> block::Block<(), crate::iil::flat::OpExprUntyped, ()> {
    // add all transitively required types to the set too
    loop {
        let new_types: HashSet<_> = types_to_build
            .iter()
            .flat_map(TypeToSpv::prerequisites)
            .filter(|new_type| !types_to_build.contains(new_type))
            .collect();
        if new_types.is_empty() {
            break;
        }
        types_to_build.extend(new_types);
    }

    let mut built_types = HashMap::<types::Type, block::BlockLocalRef>::new();
    blockctx.new_block(|blockbuilder, blockctx| {
        while !types_to_build.is_empty() {
            let candidate = types_to_build
                .iter()
                .find(|t| {
                    t.prerequisites()
                        .all(|prereq| built_types.contains_key(&prereq))
                })
                // FIXME do this without panicking
                .expect("should be able to build at least one type")
                .clone();
            let built = candidate.to_direct_instruction(&built_types);
            let built = built.expect("should be able to build instruction for type if we've already built all its prerequisites");
            assert!(types_to_build.remove(&candidate));
            built_types.insert(candidate, blockbuilder.push_valued_local(built));
        }
    })
}

#[rstest]
#[case("u32")]
#[case("u32v4")]
#[case("u32m4x3")]
#[case("u32m4x3 u32m4x4 u32m3x3")]
fn test_types_to_asm(#[case] types_src: &str) {
    use chumsky::{Parser, prelude::*};

    let types: HashSet<_> = crate::lexer::r#type()
        .separated_by(just(' ').repeated().at_least(1))
        .collect()
        .parse(types_src)
        .into_result()
        .unwrap();

    let mut blockctx = block::Ctx::new();

    let _ = types_to_asm(types, &mut blockctx);
}
