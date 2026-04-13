//! Scope management for name resolution.
//!
//! Scopes form a tree: each scope has an optional parent, and name lookup
//! walks up the tree until a binding is found or the root is reached.

use std::collections::HashMap;

use crate::hir::nodes::DefId;

/// Unique identifier for a scope.
pub type ScopeId = u32;

/// The kind of scope — determines what names are available and what
/// control flow constructs are valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// Top-level module scope
    Module,
    /// Inside a class body
    Class,
    /// Inside a trait body
    Trait,
    /// Inside an impl block
    Impl,
    /// Inside a function body
    Function,
    /// A generic block `do...end` or `{ }`
    Block,
    /// Inside a loop (enables `break` and `continue`)
    Loop,
    /// Inside a match expression
    Match,
    /// Inside a closure
    Closure,
}

/// A single lexical scope.
#[derive(Debug)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub kind: ScopeKind,
    /// Name → DefId bindings in this scope.
    bindings: HashMap<String, DefId>,
    /// Type name → DefId bindings (for classes, structs, enums, traits, type aliases).
    type_bindings: HashMap<String, DefId>,
}

impl Scope {
    fn new(id: ScopeId, parent: Option<ScopeId>, kind: ScopeKind) -> Self {
        Self {
            id,
            parent,
            kind,
            bindings: HashMap::new(),
            type_bindings: HashMap::new(),
        }
    }

    /// Insert a value binding into this scope.
    /// Returns the previous DefId if the name was already bound (shadowing).
    pub fn insert(&mut self, name: String, def_id: DefId) -> Option<DefId> {
        self.bindings.insert(name, def_id)
    }

    /// Insert a type binding into this scope.
    pub fn insert_type(&mut self, name: String, def_id: DefId) -> Option<DefId> {
        self.type_bindings.insert(name, def_id)
    }

    /// Look up a value name in this scope only (not parents).
    pub fn lookup_local(&self, name: &str) -> Option<DefId> {
        self.bindings.get(name).copied()
    }

    /// Look up a type name in this scope only.
    pub fn lookup_type_local(&self, name: &str) -> Option<DefId> {
        self.type_bindings.get(name).copied()
    }
}

/// The scope stack manages all scopes during name resolution.
#[derive(Debug)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
    /// The current (innermost) scope.
    current: ScopeId,
}

impl ScopeStack {
    pub fn new() -> Self {
        // Start with a global module scope
        let global = Scope::new(0, None, ScopeKind::Module);
        Self {
            scopes: vec![global],
            current: 0,
        }
    }

    /// Push a new child scope and make it current.
    pub fn push(&mut self, kind: ScopeKind) -> ScopeId {
        let id = self.scopes.len() as ScopeId;
        let scope = Scope::new(id, Some(self.current), kind);
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Pop the current scope, returning to the parent.
    pub fn pop(&mut self) {
        if let Some(parent) = self.scopes[self.current as usize].parent {
            self.current = parent;
        }
    }

    /// Get the current scope ID.
    pub fn current_id(&self) -> ScopeId {
        self.current
    }

    /// Get the current scope kind.
    pub fn current_kind(&self) -> ScopeKind {
        self.scopes[self.current as usize].kind
    }

    /// Insert a value binding into the current scope.
    pub fn insert(&mut self, name: String, def_id: DefId) -> Option<DefId> {
        self.scopes[self.current as usize].insert(name, def_id)
    }

    /// Insert a type binding into the current scope.
    pub fn insert_type(&mut self, name: String, def_id: DefId) -> Option<DefId> {
        self.scopes[self.current as usize].insert_type(name, def_id)
    }

    /// Look up a value name, walking up the scope chain.
    pub fn lookup(&self, name: &str) -> Option<DefId> {
        let mut scope_id = Some(self.current);
        while let Some(id) = scope_id {
            let scope = &self.scopes[id as usize];
            if let Some(def_id) = scope.lookup_local(name) {
                return Some(def_id);
            }
            scope_id = scope.parent;
        }
        None
    }

    /// Look up a type name, walking up the scope chain.
    pub fn lookup_type(&self, name: &str) -> Option<DefId> {
        let mut scope_id = Some(self.current);
        while let Some(id) = scope_id {
            let scope = &self.scopes[id as usize];
            if let Some(def_id) = scope.lookup_type_local(name) {
                return Some(def_id);
            }
            scope_id = scope.parent;
        }
        None
    }

    /// Check if we're currently inside a loop (for break/continue validation).
    pub fn in_loop(&self) -> bool {
        let mut scope_id = Some(self.current);
        while let Some(id) = scope_id {
            let scope = &self.scopes[id as usize];
            if scope.kind == ScopeKind::Loop {
                return true;
            }
            // Stop at function boundary — loops don't cross functions
            if scope.kind == ScopeKind::Function || scope.kind == ScopeKind::Closure {
                return false;
            }
            scope_id = scope.parent;
        }
        false
    }

    /// Check if we're inside a function (for return validation).
    pub fn in_function(&self) -> bool {
        let mut scope_id = Some(self.current);
        while let Some(id) = scope_id {
            let scope = &self.scopes[id as usize];
            if scope.kind == ScopeKind::Function || scope.kind == ScopeKind::Closure {
                return true;
            }
            scope_id = scope.parent;
        }
        false
    }

    /// Find the nearest enclosing class/impl scope's DefId for `self` resolution.
    pub fn enclosing_type_scope(&self) -> Option<ScopeKind> {
        let mut scope_id = Some(self.current);
        while let Some(id) = scope_id {
            let scope = &self.scopes[id as usize];
            if matches!(scope.kind, ScopeKind::Class | ScopeKind::Impl | ScopeKind::Trait) {
                return Some(scope.kind);
            }
            scope_id = scope.parent;
        }
        None
    }

    /// Get a reference to a scope by ID.
    pub fn get_scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id as usize]
    }
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}
