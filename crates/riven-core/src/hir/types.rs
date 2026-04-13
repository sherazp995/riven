//! Type representation for the Riven type system.
//!
//! Every type in Riven is represented as a `Ty`. During type inference,
//! unknown types use `Ty::Infer(TypeId)` which gets resolved through
//! unification. After type checking, all `Infer` types must be resolved
//! to concrete types.

use std::fmt;

/// Unique identifier for type variables during inference.
pub type TypeId = u32;

/// A reference to a trait, optionally with generic arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitRef {
    pub name: String,
    pub generic_args: Vec<Ty>,
}

impl fmt::Display for TraitRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.generic_args.is_empty() {
            write!(f, "[")?;
            for (i, arg) in self.generic_args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            write!(f, "]")?;
        }
        Ok(())
    }
}

/// The core type representation for Riven.
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    // Primitives
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    ISize,
    USize,
    Float,
    Float32,
    Float64,
    Bool,
    Char,
    /// `()` — the unit type
    Unit,
    /// `!` — the never/bottom type (subtype of everything)
    Never,

    // String types
    /// Owned `String` (heap-allocated, growable)
    String,
    /// `&str` — borrowed string slice
    Str,

    // Composite types
    /// `(T, U, V)` — fixed-size heterogeneous tuple
    Tuple(Vec<Ty>),
    /// `[T; N]` — fixed-size array
    Array(Box<Ty>, usize),
    /// `Vec[T]` — dynamic, heap-allocated
    Vec(Box<Ty>),
    /// `Hash[K, V]` — key-value map
    Hash(Box<Ty>, Box<Ty>),
    /// `Set[T]`
    Set(Box<Ty>),

    // Option and Result
    /// `Option[T]`
    Option(Box<Ty>),
    /// `Result[T, E]`
    Result(Box<Ty>, Box<Ty>),

    // References
    /// `&T` — immutable borrow
    Ref(Box<Ty>),
    /// `&mut T` — mutable borrow
    RefMut(Box<Ty>),
    /// `&'a T` — immutable borrow with explicit lifetime
    RefLifetime(std::string::String, Box<Ty>),
    /// `&'a mut T` — mutable borrow with explicit lifetime
    RefMutLifetime(std::string::String, Box<Ty>),

    // User-defined types
    Class {
        name: std::string::String,
        generic_args: Vec<Ty>,
    },
    Struct {
        name: std::string::String,
        generic_args: Vec<Ty>,
    },
    Enum {
        name: std::string::String,
        generic_args: Vec<Ty>,
    },

    // Trait-related
    /// `impl Trait` — static dispatch, structural satisfaction OK
    ImplTrait(Vec<TraitRef>),
    /// `dyn Trait` — dynamic dispatch, requires explicit impl
    DynTrait(Vec<TraitRef>),

    // Function types
    Fn {
        params: Vec<Ty>,
        ret: Box<Ty>,
    },
    FnMut {
        params: Vec<Ty>,
        ret: Box<Ty>,
    },
    FnOnce {
        params: Vec<Ty>,
        ret: Box<Ty>,
    },

    /// Unknown type to be resolved during inference
    Infer(TypeId),

    /// Generic type parameter: `T`, `T: Bound`
    TypeParam {
        name: std::string::String,
        bounds: Vec<TraitRef>,
    },

    /// Type alias target (transparent)
    Alias {
        name: std::string::String,
        target: Box<Ty>,
    },

    /// Newtype wrapper (opaque)
    Newtype {
        name: std::string::String,
        inner: Box<Ty>,
    },

    /// Raw immutable pointer: `*T` (C's `const T*`)
    RawPtr(Box<Ty>),

    /// Raw mutable pointer: `*mut T` (C's `T*`)
    RawPtrMut(Box<Ty>),

    /// Opaque void pointer: `*Void` (C's `const void*`)
    RawPtrVoid,

    /// Opaque mutable void pointer: `*mut Void` (C's `void*`)
    RawPtrMutVoid,

    /// Placeholder for error recovery — allows type checking to continue
    Error,
}

/// Metadata about a type's properties.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    pub is_copy: bool,
    pub is_drop: bool,
    pub size: Option<usize>,
    pub alignment: Option<usize>,
}

/// Whether a value is copied or moved on assignment/passing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveSemantics {
    Copy,
    Move,
}

impl Ty {
    /// Returns true if this type has Copy semantics.
    ///
    /// Copy types: all integers, floats, Bool, Char, Unit, references (&T, &str),
    /// ranges, and tuples where all elements are Copy.
    pub fn is_copy(&self) -> bool {
        match self {
            // Primitives are always Copy
            Ty::Int | Ty::Int8 | Ty::Int16 | Ty::Int32 | Ty::Int64
            | Ty::UInt | Ty::UInt8 | Ty::UInt16 | Ty::UInt32 | Ty::UInt64
            | Ty::ISize | Ty::USize
            | Ty::Float | Ty::Float32 | Ty::Float64
            | Ty::Bool | Ty::Char | Ty::Unit => true,

            // Never is Copy (vacuously — you can never have a Never value)
            Ty::Never => true,

            // Immutable references are Copy
            Ty::Ref(_) | Ty::RefLifetime(_, _) => true,
            // &str is Copy (it's a borrowed reference)
            Ty::Str => true,

            // Tuples are Copy if all elements are Copy
            Ty::Tuple(elems) => elems.iter().all(|e| e.is_copy()),

            // Arrays are Copy if element type is Copy
            Ty::Array(elem, _) => elem.is_copy(),

            // Raw pointers are Copy (like in Rust)
            Ty::RawPtr(_) | Ty::RawPtrMut(_) | Ty::RawPtrVoid | Ty::RawPtrMutVoid => true,

            // Error type is treated as Copy for error recovery
            Ty::Error => true,

            // Everything else is Move
            _ => false,
        }
    }

    /// Returns true if this type has Move semantics.
    pub fn is_move(&self) -> bool {
        !self.is_copy()
    }

    /// Returns the move semantics for this type.
    pub fn move_semantics(&self) -> MoveSemantics {
        if self.is_copy() {
            MoveSemantics::Copy
        } else {
            MoveSemantics::Move
        }
    }

    /// Returns true if this is any kind of reference.
    pub fn is_ref(&self) -> bool {
        matches!(
            self,
            Ty::Ref(_) | Ty::RefMut(_) | Ty::RefLifetime(_, _) | Ty::RefMutLifetime(_, _) | Ty::Str
        )
    }

    /// Returns true if this is a mutable reference.
    pub fn is_mut_ref(&self) -> bool {
        matches!(self, Ty::RefMut(_) | Ty::RefMutLifetime(_, _))
    }

    /// Returns true if this is an immutable reference.
    pub fn is_immut_ref(&self) -> bool {
        matches!(self, Ty::Ref(_) | Ty::RefLifetime(_, _) | Ty::Str)
    }

    /// Returns the inner type if this is a reference, otherwise None.
    pub fn deref_ty(&self) -> Option<&Ty> {
        match self {
            Ty::Ref(inner) | Ty::RefMut(inner) => Some(inner),
            Ty::RefLifetime(_, inner) | Ty::RefMutLifetime(_, inner) => Some(inner),
            _ => None,
        }
    }

    /// Returns true if this is an unresolved inference variable.
    pub fn is_infer(&self) -> bool {
        matches!(self, Ty::Infer(_))
    }

    /// Returns true if this is the error sentinel type.
    pub fn is_error(&self) -> bool {
        matches!(self, Ty::Error)
    }

    /// Returns true if this is the Never (bottom) type.
    pub fn is_never(&self) -> bool {
        matches!(self, Ty::Never)
    }

    /// Returns true if this is a numeric type (integer or float).
    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    /// Returns true if this is any integer type.
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Ty::Int | Ty::Int8 | Ty::Int16 | Ty::Int32 | Ty::Int64
            | Ty::UInt | Ty::UInt8 | Ty::UInt16 | Ty::UInt32 | Ty::UInt64
            | Ty::ISize | Ty::USize
        )
    }

    /// Returns true if this is any float type.
    pub fn is_float(&self) -> bool {
        matches!(self, Ty::Float | Ty::Float32 | Ty::Float64)
    }

    /// Returns true if this is a signed integer type.
    pub fn is_signed_integer(&self) -> bool {
        matches!(
            self,
            Ty::Int | Ty::Int8 | Ty::Int16 | Ty::Int32 | Ty::Int64 | Ty::ISize
        )
    }

    /// Returns true if this is an unsigned integer type.
    pub fn is_unsigned_integer(&self) -> bool {
        matches!(
            self,
            Ty::UInt | Ty::UInt8 | Ty::UInt16 | Ty::UInt32 | Ty::UInt64 | Ty::USize
        )
    }

    /// Returns the bit width of a numeric type, or None.
    pub fn bit_width(&self) -> Option<u32> {
        match self {
            Ty::Int8 | Ty::UInt8 => Some(8),
            Ty::Int16 | Ty::UInt16 => Some(16),
            Ty::Int32 | Ty::UInt32 | Ty::Float32 => Some(32),
            Ty::Int64 | Ty::UInt64 | Ty::Float64 => Some(64),
            Ty::Int | Ty::UInt | Ty::Float => Some(64), // defaults
            Ty::ISize | Ty::USize => Some(64),           // assume 64-bit platform
            _ => None,
        }
    }

    /// Returns true if this is an Option type.
    pub fn is_option(&self) -> bool {
        matches!(self, Ty::Option(_))
    }

    /// Returns true if this is a Result type.
    pub fn is_result(&self) -> bool {
        matches!(self, Ty::Result(_, _))
    }

    /// Returns the user-visible name of this type.
    pub fn type_name(&self) -> std::string::String {
        format!("{}", self)
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Int => write!(f, "Int"),
            Ty::Int8 => write!(f, "Int8"),
            Ty::Int16 => write!(f, "Int16"),
            Ty::Int32 => write!(f, "Int32"),
            Ty::Int64 => write!(f, "Int64"),
            Ty::UInt => write!(f, "UInt"),
            Ty::UInt8 => write!(f, "UInt8"),
            Ty::UInt16 => write!(f, "UInt16"),
            Ty::UInt32 => write!(f, "UInt32"),
            Ty::UInt64 => write!(f, "UInt64"),
            Ty::ISize => write!(f, "ISize"),
            Ty::USize => write!(f, "USize"),
            Ty::Float => write!(f, "Float"),
            Ty::Float32 => write!(f, "Float32"),
            Ty::Float64 => write!(f, "Float64"),
            Ty::Bool => write!(f, "Bool"),
            Ty::Char => write!(f, "Char"),
            Ty::Unit => write!(f, "()"),
            Ty::Never => write!(f, "Never"),
            Ty::String => write!(f, "String"),
            Ty::Str => write!(f, "&str"),
            Ty::Tuple(elems) => {
                write!(f, "(")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                if elems.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            Ty::Array(elem, size) => write!(f, "[{}; {}]", elem, size),
            Ty::Vec(elem) => write!(f, "Vec[{}]", elem),
            Ty::Hash(k, v) => write!(f, "Hash[{}, {}]", k, v),
            Ty::Set(elem) => write!(f, "Set[{}]", elem),
            Ty::Option(inner) => write!(f, "Option[{}]", inner),
            Ty::Result(ok, err) => write!(f, "Result[{}, {}]", ok, err),
            Ty::Ref(inner) => write!(f, "&{}", inner),
            Ty::RefMut(inner) => write!(f, "&mut {}", inner),
            Ty::RefLifetime(lt, inner) => write!(f, "&'{} {}", lt, inner),
            Ty::RefMutLifetime(lt, inner) => write!(f, "&'{} mut {}", lt, inner),
            Ty::Class { name, generic_args } | Ty::Struct { name, generic_args } | Ty::Enum { name, generic_args } => {
                write!(f, "{}", name)?;
                if !generic_args.is_empty() {
                    write!(f, "[")?;
                    for (i, a) in generic_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", a)?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
            Ty::ImplTrait(bounds) => {
                write!(f, "impl ")?;
                for (i, b) in bounds.iter().enumerate() {
                    if i > 0 {
                        write!(f, " + ")?;
                    }
                    write!(f, "{}", b)?;
                }
                Ok(())
            }
            Ty::DynTrait(bounds) => {
                write!(f, "dyn ")?;
                for (i, b) in bounds.iter().enumerate() {
                    if i > 0 {
                        write!(f, " + ")?;
                    }
                    write!(f, "{}", b)?;
                }
                Ok(())
            }
            Ty::Fn { params, ret } => {
                write!(f, "Fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Ty::FnMut { params, ret } => {
                write!(f, "FnMut(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Ty::FnOnce { params, ret } => {
                write!(f, "FnOnce(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Ty::Infer(id) => write!(f, "?T{}", id),
            Ty::TypeParam { name, bounds } => {
                write!(f, "{}", name)?;
                if !bounds.is_empty() {
                    write!(f, ": ")?;
                    for (i, b) in bounds.iter().enumerate() {
                        if i > 0 {
                            write!(f, " + ")?;
                        }
                        write!(f, "{}", b)?;
                    }
                }
                Ok(())
            }
            Ty::Alias { name, .. } => write!(f, "{}", name),
            Ty::Newtype { name, .. } => write!(f, "{}", name),
            Ty::RawPtr(inner) => write!(f, "*{}", inner),
            Ty::RawPtrMut(inner) => write!(f, "*mut {}", inner),
            Ty::RawPtrVoid => write!(f, "*Void"),
            Ty::RawPtrMutVoid => write!(f, "*mut Void"),
            Ty::Error => write!(f, "<error>"),
        }
    }
}
