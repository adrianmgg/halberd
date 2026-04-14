use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(usize);

impl ScopeId {
    fn into_idx(self) -> usize { self.0 }

    fn from_idx(idx: usize) -> Self { Self(idx) }
}

pub struct Universe {
    scopes: Vec<Scope>,
    root_scope_id: ScopeId,
}

impl Universe {
    pub fn new() -> Self {
        let root_scope_id = ScopeId::from_idx(0);
        let scopes = vec![Scope {
            parent: None,
            items: Default::default(),
        }];
        Self {
            scopes,
            root_scope_id,
        }
    }

    fn get_just_scope(&self, id: ScopeId) -> &Scope {
        self.scopes
            .get(id.into_idx())
            .expect("Ids should always be valid since we're the only one who can give them out.")
    }

    fn get_just_scope_mut(&mut self, id: ScopeId) -> &mut Scope {
        self.scopes
            .get_mut(id.into_idx())
            .expect("Ids should always be valid since we're the only one who can give them out.")
    }

    pub fn get_scope(&self, id: ScopeId) -> ScopeRef<'_> {
        ScopeRef {
            universe: self,
            scope: id,
        }
    }

    pub fn get_scope_mut(&mut self, id: ScopeId) -> ScopeRefMut<'_> {
        ScopeRefMut {
            universe: self,
            scope: id,
        }
    }

    pub fn root_scope(&self) -> ScopeRef<'_> { self.get_scope(self.root_scope_id) }

    pub fn root_scope_mut(&mut self) -> ScopeRefMut<'_> { self.get_scope_mut(self.root_scope_id) }

    fn scope_lookup(&self, scope_id: ScopeId, key: &str) -> Option<&ScopeItem> {
        let mut scope = self.get_just_scope(scope_id);
        loop {
            // check for matching item in current layer
            if let Some(item) = scope.items.get(key) {
                return Some(item);
            }
            // no match here, try our parent if any, or give up otherwise
            match scope.parent {
                Some(id) => scope = self.get_just_scope(id),
                None => return None,
            }
        }
    }

    fn new_scope(&mut self, parent_id: ScopeId) -> ScopeId {
        let id = ScopeId::from_idx(self.scopes.len());
        self.scopes.push(Scope {
            parent: Some(parent_id),
            items: Default::default(),
        });
        id
    }
}

pub struct Scope {
    // TODO maybe make id nonzero so we get more compact representations of these?
    //      losing a single value of 2^64 possible ids in order to cut down the size
    //      of every Option<Id> is probably a worthwhile tradeoff.
    parent: Option<ScopeId>,
    items: HashMap<Box<str>, ScopeItem>,
}

pub struct ScopeRef<'a> {
    universe: &'a Universe,
    scope: ScopeId,
}

pub struct ScopeRefMut<'a> {
    universe: &'a mut Universe,
    scope: ScopeId,
}

impl<'a> ScopeRef<'a> {
    pub fn lookup(&self, key: &str) -> Option<&ScopeItem> {
        self.universe.scope_lookup(self.scope, key)
    }
}

impl<'a> ScopeRefMut<'a> {
    pub fn lookup(&self, key: &str) -> Option<&ScopeItem> {
        self.universe.scope_lookup(self.scope, key)
    }

    pub fn new_subscope(&mut self) -> ScopeId { self.universe.new_scope(self.scope) }
}

pub enum ScopeItem {
    Variable,
}
