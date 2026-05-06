use std::borrow::Cow;

use num_bigint::BigInt;
use num_rational::BigRational;

use crate::{
    iil::{
        self,
        block::{self, Renumberable},
        flat::IilOpExpr as _,
    },
    spv::operand_kind,
    types,
    util::{impl_conversion_enum_variant, matches_opt},
};

pub trait FlattenableToBlock {
    fn flatten(self, ctx: &mut block::Ctx) -> crate::iil::h::Block;
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Constant {
    Int { r#type: types::Integer, value: BigInt },
    Float { r#type: types::Float, value: BigRational },
    Bool { value: bool },
}

#[derive(Debug)]
pub enum BlockLocalExpr {
    Op(iil::flat::OpExpr),
    OpUntyped(iil::flat::OpExprUntyped),
    Block(Box<Block>),
    Constant(Constant),
    Ref(block::BlockLocalRef),
}
impl_conversion_enum_variant!(BlockLocalExpr::{Op(iil::flat::OpExpr), Block(Box<Block>), Constant(Constant), Ref(block::BlockLocalRef)});

impl Renumberable for BlockLocalExpr {
    fn renumber(&mut self, from: block::BlockLocalRef, to: block::BlockLocalRef) -> bool {
        match self {
            BlockLocalExpr::Op(op_expr) => op_expr.renumber(from, to),
            BlockLocalExpr::OpUntyped(op) => op.renumber(from, to),
            BlockLocalExpr::Block(block) => block.renumber(from, to),
            BlockLocalExpr::Constant(constant) => false,
            BlockLocalExpr::Ref(r) => r.renumber(from, to),
        }
    }
}

#[derive(Debug)]
pub enum FlatBlockLocalExpr {
    Op(iil::flat::OpExpr),
    OpUntyped(iil::flat::OpExprUntyped),
    Constant(Constant),
    Ref(block::BlockLocalRef),
}
impl_conversion_enum_variant!(FlatBlockLocalExpr::{Op(iil::flat::OpExpr), Constant(Constant), Ref(block::BlockLocalRef)});

pub type BlockLocalVoid = iil::flat::OpVoid;
pub type BlockTerminal = BlockLocalExpr;
pub type Block = block::Block<BlockLocalVoid, BlockLocalExpr, Option<BlockTerminal>>;
pub type FlatBlock = block::Block<BlockLocalVoid, FlatBlockLocalExpr, Option<FlatBlockLocalExpr>>;

#[derive(Debug)]
pub struct Function {
    pub control: enumset::EnumSet<operand_kind::FunctionControl>,
    pub r#type: types::Function,
    pub body: Block,
    // FIXME temp. for hardcoded demo stuff
    pub is_main: bool,
}

#[derive(Debug)]
pub struct FlatFunction {
    pub control: enumset::EnumSet<operand_kind::FunctionControl>,
    pub r#type: types::Function,
    pub body: FlatBlock,
    // FIXME temp. for hardcoded demo stuff
    pub is_main: bool,
}

impl FlatFunction {
    pub(crate) fn types_referenced(&self) -> impl Iterator<Item = Cow<'_, types::Type>> {
        use std::iter::{chain, once};
        let body_locals = self.body.locals().filter_map(|(_, op)| match op {
            block::BlockLocal::Void(op_void) => None,
            block::BlockLocal::Valued(expr) => match expr {
                FlatBlockLocalExpr::Constant(_)
                | FlatBlockLocalExpr::Ref(_)
                | FlatBlockLocalExpr::OpUntyped(_) => None,
                FlatBlockLocalExpr::Op(op_expr) => Some(Cow::Borrowed(op_expr.ret_type())),
            },
        });
        let body_terminal = self.body.terminal().as_ref().and_then(|a| match a {
            FlatBlockLocalExpr::Constant(_)
            | FlatBlockLocalExpr::Ref(_)
            | FlatBlockLocalExpr::OpUntyped(_) => None,
            FlatBlockLocalExpr::Op(op_expr) => Some(Cow::Borrowed(op_expr.ret_type())),
        });
        let overall = Cow::Owned(self.r#type.clone().into());
        chain(chain(body_locals, once(overall)), body_terminal)
    }

    pub(crate) fn constants_referenced(&self) -> impl Iterator<Item = &Constant> {
        use std::iter::{chain, once};
        let body_constants = self
            .body
            .locals_valued_only()
            .filter_map(|(_, local)| matches_opt!(local, FlatBlockLocalExpr::Constant(c) => c));
        let terminal_constant_maybe = self
            .body
            .terminal()
            .as_ref()
            .and_then(|terminal| matches_opt!(terminal, FlatBlockLocalExpr::Constant(c) => c));
        chain(body_constants, terminal_constant_maybe)
    }
}
