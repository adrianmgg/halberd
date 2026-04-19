use std::{
    convert::Infallible,
    fmt::{Debug, Display},
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

    pub fn locals(
        &self,
    ) -> impl Iterator<Item = (BlockLocalRef, &BlockLocal<VoidLocal, ValuedLocal>)> {
        self.locals
            .iter()
            .enumerate()
            .map(|(i, local)| (BlockLocalRef { block: self.id, local: i }, local))
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
