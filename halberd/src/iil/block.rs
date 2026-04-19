use std::{
    convert::Infallible,
    fmt::{Debug, Display},
    ops::DerefMut,
};

use unwrap_infallible::UnwrapInfallible as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(u64);

impl Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Display::fmt(&self.0, f) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BlockLocalRef {
    block: BlockId,
    local: usize,
}

impl Display for BlockLocalRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}.{}", self.block, self.local)
    }
}

impl Debug for BlockLocalRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("BlockLocalRef({self})"))
    }
}

pub struct Ctx {
    next_id: u64,
}

impl Ctx {
    pub fn new() -> Self { Self { next_id: 0 } }

    fn new_id(&mut self) -> BlockId {
        let v = self.next_id;
        self.next_id += 1;
        BlockId(v)
    }

    pub fn new_block<VoidLocal, ValuedLocal, Terminal, F>(
        &mut self,
        f: F,
    ) -> Block<VoidLocal, ValuedLocal, Terminal>
    where
        F: FnOnce(&mut BlockBuilder<VoidLocal, ValuedLocal>, &mut Self) -> Terminal,
    {
        self.try_new_block(|a, b| Result::<_, Infallible>::Ok(f(a, b)))
            .unwrap_infallible()
    }

    pub fn try_new_block<VoidLocal, ValuedLocal, Terminal, E, F>(
        &mut self,
        f: F,
    ) -> Result<Block<VoidLocal, ValuedLocal, Terminal>, E>
    where
        F: FnOnce(&mut BlockBuilder<VoidLocal, ValuedLocal>, &mut Self) -> Result<Terminal, E>,
    {
        let mut builder = BlockBuilder { id: self.new_id(), locals: Vec::new() };
        f(&mut builder, self).map(|terminal| Block {
            id: builder.id,
            locals: builder.locals,
            terminal,
        })
    }
}

pub struct BlockBuilder<VoidLocal, ValuedLocal> {
    id: BlockId,
    locals: Vec<BlockLocal<VoidLocal, ValuedLocal>>,
}

pub struct Block<VoidLocal, ValuedLocal, Terminal> {
    id: BlockId,
    locals: Vec<BlockLocal<VoidLocal, ValuedLocal>>,
    terminal: Terminal,
}

impl<VoidLocal, ValuedLocal, Terminal> Block<VoidLocal, ValuedLocal, Terminal> {
    pub fn id(&self) -> BlockId { self.id }

    pub fn terminal(&self) -> &Terminal { &self.terminal }

    pub fn locals(
        &self,
    ) -> impl Iterator<Item = (BlockLocalRef, &BlockLocal<VoidLocal, ValuedLocal>)> {
        self.locals
            .iter()
            .enumerate()
            .map(|(i, local)| (BlockLocalRef { block: self.id, local: i }, local))
    }

    pub fn into_parts(
        self,
    ) -> (
        impl Iterator<Item = (BlockLocalRef, BlockLocal<VoidLocal, ValuedLocal>)>,
        Terminal,
    ) {
        let id = self.id;
        let locals = self
            .locals
            .into_iter()
            .enumerate()
            .map(move |(i, local)| (BlockLocalRef { block: id, local: i }, local));
        (locals, self.terminal)
    }

    pub fn map<NewVoidLocal, NewValuedLocal, NewTerminal, MapVoid, MapValued, MapTerminal>(
        self,
        map_void: MapVoid,
        map_valued: MapValued,
        map_terminal: MapTerminal,
    ) -> Block<NewVoidLocal, NewValuedLocal, NewTerminal>
    where
        MapVoid: Fn(VoidLocal) -> NewVoidLocal,
        MapValued: Fn(ValuedLocal) -> NewValuedLocal,
        MapTerminal: FnOnce(Terminal) -> NewTerminal,
    {
        Block {
            id: self.id,
            locals: self
                .locals
                .into_iter()
                .map(|local| match local {
                    BlockLocal::Void(void) => BlockLocal::Void(map_void(void)),
                    BlockLocal::Valued(valued) => BlockLocal::Valued(map_valued(valued)),
                })
                .collect(),
            terminal: map_terminal(self.terminal),
        }
    }
}

impl<VoidLocal, ValuedLocal, Terminal> Debug for Block<VoidLocal, ValuedLocal, Terminal>
where
    VoidLocal: Debug,
    ValuedLocal: Debug,
    Terminal: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Block");
        s.field("id", &self.id);
        for (i, local) in self.locals.iter().enumerate() {
            match local {
                BlockLocal::Void(local) => {
                    s.field("_", local);
                }
                BlockLocal::Valued(local) => {
                    let name = format!("{}", BlockLocalRef { block: self.id, local: i });
                    s.field(&name, &local);
                }
            }
        }
        s.field("terminal", &self.terminal).finish()
    }
}

#[derive(Clone)]
pub enum BlockLocal<Void, Valued> {
    Void(Void),
    Valued(Valued),
}

impl<Void, Valued> BlockBuilder<Void, Valued> {
    pub fn push_void_local(&mut self, local: Void) { self.locals.push(BlockLocal::Void(local)); }

    pub fn push_valued_local(&mut self, local: Valued) -> BlockLocalRef {
        let new_local_idx = self.locals.len();
        self.locals.push(BlockLocal::Valued(local));
        BlockLocalRef { block: self.id, local: new_local_idx }
    }
}

pub trait Renumberable {
    fn renumber(&mut self, from: BlockLocalRef, to: BlockLocalRef);
}

impl<Void, Valued> Renumberable for BlockLocal<Void, Valued>
where
    Void: Renumberable,
    Valued: Renumberable,
{
    fn renumber(&mut self, from: BlockLocalRef, to: BlockLocalRef) {
        match self {
            BlockLocal::Void(void) => void.renumber(from, to),
            BlockLocal::Valued(valued) => valued.renumber(from, to),
        }
    }
}

impl<Void, Valued, Terminal> Renumberable for Block<Void, Valued, Terminal>
where
    Void: Renumberable,
    Valued: Renumberable,
    Terminal: Renumberable,
{
    fn renumber(&mut self, from: BlockLocalRef, to: BlockLocalRef) {
        for local in &mut self.locals {
            local.renumber(from, to);
        }
        self.terminal.renumber(from, to);
    }
}

impl<T> Renumberable for Vec<T>
where T: Renumberable
{
    fn renumber(&mut self, from: BlockLocalRef, to: BlockLocalRef) {
        self.as_mut_slice().renumber(from, to);
    }
}

impl<T> Renumberable for [T]
where T: Renumberable
{
    fn renumber(&mut self, from: BlockLocalRef, to: BlockLocalRef) {
        for x in self.iter_mut() {
            x.renumber(from, to);
        }
    }
}

impl<T> Renumberable for Option<T>
where T: Renumberable
{
    fn renumber(&mut self, from: BlockLocalRef, to: BlockLocalRef) {
        for x in self.iter_mut() {
            x.renumber(from, to);
        }
    }
}

impl Renumberable for BlockLocalRef {
    fn renumber(&mut self, from: BlockLocalRef, to: BlockLocalRef) {
        if *self == from {
            *self = to;
        }
    }
}

/*
impl<VoidLocal, ValuedLocal, Terminal> Block<VoidLocal, ValuedLocal, Terminal>
where
    VoidLocal: Renumberable,
    ValuedLocal: Renumberable,
    Terminal: Renumberable,
{
    fn push_all(&mut self, mut other: Self) {
        let upcoming_new_indices = self.locals.len()..;
        let renumbers: Vec<_> = upcoming_new_indices
            .zip(other.locals())
            .map(|(new_idx, (old_ref, _local))| {
                (old_ref, BlockLocalRef { block: self.id, local: new_idx })
            })
            .collect();

        let (locals, terminal) = other.into_parts();
        for (n, mut local) in locals {
            // TODO should probably just make a version of renumber that takes in a function? or a
            //      list of renumbers? or a map?
            for (from, to) in &renumbers {
                local.renumber(*from, *to);
            }
            self.locals.push();
        }
    }
}
*/
