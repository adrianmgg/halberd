use std::convert::Infallible;

use unwrap_infallible::UnwrapInfallible as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(u64);

pub struct BlockLocalRef {
    block: BlockId,
    local: usize,
}

pub struct Ctx {
    next_id: u64,
}

impl Ctx {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

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
        let mut builder = BlockBuilder {
            id: self.new_id(),
            locals: Vec::new(),
        };
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

pub enum BlockLocal<Void, Valued> {
    Void(Void),
    Valued(Valued),
}

impl<Void, Valued> BlockBuilder<Void, Valued> {
    pub fn push_void_local(&mut self, local: Void) {
        self.locals.push(BlockLocal::Void(local));
    }
    pub fn push_valued_local(&mut self, local: Valued) -> BlockLocalRef {
        let new_local_idx = self.locals.len();
        self.locals.push(BlockLocal::Valued(local));
        BlockLocalRef {
            block: self.id,
            local: new_local_idx,
        }
    }
}
