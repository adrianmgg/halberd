use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(usize);

impl ScopeId {
    fn into_idx(self) -> usize { self.0 }

    fn from_idx(idx: usize) -> Self { Self(idx) }
}

#[derive(Debug)]
pub struct Universe<Item> {
    scopes: Vec<Scope<Item>>,
    root_scope_id: ScopeId,
}

impl<Item> Universe<Item> {
    pub fn new() -> Self {
        let root_scope_id = ScopeId::from_idx(0);
        let scopes = vec![Scope { parent: None, items: HashMap::default() }];
        Self { scopes, root_scope_id }
    }

    fn get_just_scope(&self, id: ScopeId) -> &Scope<Item> {
        self.scopes
            .get(id.into_idx())
            .expect("Ids should always be valid since we're the only one who can give them out.")
    }

    fn get_just_scope_mut(&mut self, id: ScopeId) -> &mut Scope<Item> {
        self.scopes
            .get_mut(id.into_idx())
            .expect("Ids should always be valid since we're the only one who can give them out.")
    }

    pub fn get_scope(&self, id: ScopeId) -> ScopeRef<'_, Item> {
        ScopeRef { universe: self, scope: id }
    }

    pub fn get_scope_mut(&mut self, id: ScopeId) -> ScopeRefMut<'_, Item> {
        ScopeRefMut { universe: self, scope: id }
    }

    pub fn root_scope_id(&self) -> ScopeId { self.root_scope_id }

    pub fn root_scope(&self) -> ScopeRef<'_, Item> { self.get_scope(self.root_scope_id) }

    pub fn root_scope_mut(&mut self) -> ScopeRefMut<'_, Item> {
        self.get_scope_mut(self.root_scope_id)
    }

    fn scope_lookup(&self, scope_id: ScopeId, key: &str) -> Option<&Item> {
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

    fn scope_lookup_and_modify<F: FnOnce(&mut Item)>(
        &mut self,
        scope_id: ScopeId,
        key: &str,
        f: F,
    ) -> bool {
        let mut scope = self.get_just_scope_mut(scope_id);
        loop {
            // check for matching item in current layer
            if let Some(item) = scope.items.get_mut(key) {
                f(item);
                return true;
            }
            // no match here, try our parent if any, or give up otherwise
            match scope.parent {
                Some(id) => scope = self.get_just_scope_mut(id),
                None => return false,
            }
        }
    }

    fn scope_insert(&mut self, scope_id: ScopeId, key: Box<str>, item: Item) {
        let mut scope = self.get_just_scope_mut(scope_id);
        scope.items.insert(key, item);
    }

    fn new_scope(&mut self, parent_id: ScopeId) -> ScopeId {
        let id = ScopeId::from_idx(self.scopes.len());
        self.scopes
            .push(Scope { parent: Some(parent_id), items: HashMap::default() });
        id
    }

    pub fn map<Item2, F: Fn(Item) -> Item2>(self, f: F) -> Universe<Item2> {
        Universe {
            scopes: self
                .scopes
                .into_iter()
                .map(|scope| Scope {
                    parent: scope.parent,
                    items: scope.items.into_iter().map(|(k, v)| (k, f(v))).collect(),
                })
                .collect(),
            root_scope_id: self.root_scope_id,
        }
    }
}

#[derive(Debug)]
pub struct Scope<Item> {
    // TODO maybe make id nonzero so we get more compact representations of these?
    //      losing a single value of 2^64 possible ids in order to cut down the size
    //      of every Option<Id> is probably a worthwhile tradeoff.
    parent: Option<ScopeId>,
    items: HashMap<Box<str>, Item>,
}

pub struct ScopeRef<'a, Item> {
    universe: &'a Universe<Item>,
    scope: ScopeId,
}

pub struct ScopeRefMut<'a, Item> {
    universe: &'a mut Universe<Item>,
    scope: ScopeId,
}

impl<Item> ScopeRef<'_, Item> {
    pub fn lookup(&self, key: &str) -> Option<&Item> { self.universe.scope_lookup(self.scope, key) }
}

impl<Item> ScopeRefMut<'_, Item> {
    pub fn lookup(&self, key: &str) -> Option<&Item> { self.universe.scope_lookup(self.scope, key) }

    pub fn lookup_and_modify<F: FnOnce(&mut Item)>(&mut self, key: &str, f: F) -> bool {
        self.universe.scope_lookup_and_modify(self.scope, key, f)
    }

    pub fn new_subscope(&mut self) -> ScopeId { self.universe.new_scope(self.scope) }

    pub fn insert<S: Into<Box<str>>>(&mut self, key: S, item: Item) {
        self.universe.scope_insert(self.scope, key.into(), item);
    }
}
