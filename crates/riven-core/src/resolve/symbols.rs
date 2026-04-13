//! Symbol table — stores all definitions in the program.
//!
//! Every named entity (variable, function, class, field, etc.) gets a unique
//! DefId and a corresponding Definition entry in the symbol table.

use crate::hir::nodes::{DefId, HirSelfMode};
use crate::hir::types::{Ty, TraitRef};
use crate::lexer::token::Span;
use crate::parser::ast::Visibility;

/// The signature of a function or method.
#[derive(Debug, Clone)]
pub struct FnSignature {
    pub self_mode: Option<HirSelfMode>,
    pub is_class_method: bool,
    pub generic_params: Vec<GenericParamInfo>,
    pub params: Vec<ParamInfo>,
    pub return_ty: Ty,
}

#[derive(Debug, Clone)]
pub struct GenericParamInfo {
    pub name: String,
    pub bounds: Vec<TraitRef>,
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
    pub ty: Ty,
    pub auto_assign: bool,
}

/// Information about a class definition.
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub generic_params: Vec<GenericParamInfo>,
    pub parent: Option<DefId>,
    pub fields: Vec<DefId>,
    pub methods: Vec<DefId>,
}

/// Information about a struct definition.
#[derive(Debug, Clone)]
pub struct StructInfo {
    pub generic_params: Vec<GenericParamInfo>,
    pub fields: Vec<DefId>,
    pub derive_traits: Vec<String>,
}

/// Information about an enum definition.
#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub generic_params: Vec<GenericParamInfo>,
    pub variants: Vec<DefId>,
}

/// Information about a trait definition.
#[derive(Debug, Clone)]
pub struct TraitInfo {
    pub generic_params: Vec<GenericParamInfo>,
    pub super_traits: Vec<TraitRef>,
    pub required_methods: Vec<String>,
    pub default_methods: Vec<String>,
    pub assoc_types: Vec<String>,
}

/// The kind of definition — what this name refers to.
#[derive(Debug, Clone)]
pub enum DefKind {
    Variable {
        mutable: bool,
        ty: Ty,
    },
    Function {
        signature: FnSignature,
    },
    Class {
        info: ClassInfo,
    },
    Struct {
        info: StructInfo,
    },
    Enum {
        info: EnumInfo,
    },
    EnumVariant {
        parent: DefId,
        variant_idx: usize,
        kind: VariantDefKind,
    },
    Trait {
        info: TraitInfo,
    },
    TypeAlias {
        target: Ty,
    },
    Newtype {
        inner: Ty,
    },
    TypeParam {
        bounds: Vec<TraitRef>,
    },
    Module {
        items: Vec<DefId>,
    },
    Field {
        parent: DefId,
        ty: Ty,
        index: usize,
    },
    Method {
        parent: DefId,
        signature: FnSignature,
    },
    Const {
        ty: Ty,
    },
    /// A parameter in a function or closure
    Param {
        ty: Ty,
        auto_assign: bool,
    },
    /// Self reference inside a class/impl
    SelfValue {
        ty: Ty,
    },
}

/// Kind of enum variant (for construction checking).
#[derive(Debug, Clone)]
pub enum VariantDefKind {
    Unit,
    Tuple(Vec<Ty>),
    Struct(Vec<(String, Ty)>),
}

/// A single definition in the symbol table.
#[derive(Debug, Clone)]
pub struct Definition {
    pub id: DefId,
    pub name: String,
    pub kind: DefKind,
    pub visibility: Visibility,
    pub span: Span,
}

/// The symbol table: stores all definitions indexed by DefId.
#[derive(Debug)]
pub struct SymbolTable {
    definitions: Vec<Definition>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            definitions: Vec::new(),
        }
    }

    /// Allocate a new definition and return its DefId.
    pub fn define(
        &mut self,
        name: String,
        kind: DefKind,
        visibility: Visibility,
        span: Span,
    ) -> DefId {
        let id = self.definitions.len() as DefId;
        self.definitions.push(Definition {
            id,
            name,
            kind,
            visibility,
            span,
        });
        id
    }

    /// Look up a definition by DefId.
    pub fn get(&self, id: DefId) -> Option<&Definition> {
        self.definitions.get(id as usize)
    }

    /// Get a mutable reference to a definition.
    pub fn get_mut(&mut self, id: DefId) -> Option<&mut Definition> {
        self.definitions.get_mut(id as usize)
    }

    /// Get the total number of definitions.
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Check if the symbol table is empty.
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Iterate over all definitions.
    pub fn iter(&self) -> impl Iterator<Item = &Definition> {
        self.definitions.iter()
    }

    /// Update the type of a variable/field/param definition.
    pub fn update_ty(&mut self, id: DefId, new_ty: Ty) {
        if let Some(def) = self.definitions.get_mut(id as usize) {
            match &mut def.kind {
                DefKind::Variable { ty, .. } => *ty = new_ty,
                DefKind::Field { ty, .. } => *ty = new_ty,
                DefKind::Param { ty, .. } => *ty = new_ty,
                DefKind::SelfValue { ty } => *ty = new_ty,
                DefKind::Const { ty } => *ty = new_ty,
                _ => {}
            }
        }
    }

    /// Get the type associated with a definition, if applicable.
    pub fn def_ty(&self, id: DefId) -> Option<Ty> {
        self.get(id).and_then(|def| match &def.kind {
            DefKind::Variable { ty, .. } => Some(ty.clone()),
            DefKind::Field { ty, .. } => Some(ty.clone()),
            DefKind::Param { ty, .. } => Some(ty.clone()),
            DefKind::SelfValue { ty } => Some(ty.clone()),
            DefKind::Const { ty } => Some(ty.clone()),
            DefKind::Function { signature } => Some(Ty::Fn {
                params: signature.params.iter().map(|p| p.ty.clone()).collect(),
                ret: Box::new(signature.return_ty.clone()),
            }),
            DefKind::Method { signature, .. } => Some(Ty::Fn {
                params: signature.params.iter().map(|p| p.ty.clone()).collect(),
                ret: Box::new(signature.return_ty.clone()),
            }),
            _ => None,
        })
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
