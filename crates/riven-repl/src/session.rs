//! REPL session state management.
//!
//! `ReplSession` owns all cumulative state for the REPL. It grows with
//! each successfully executed input.

use std::path::PathBuf;

use riven_core::parser::ast::{FuncDef, LetBinding, TopLevelItem};

use crate::env::ReplEnv;
use crate::jit::JITCodeGen;

/// Complete state for a REPL session.
pub struct ReplSession {
    /// All source inputs so far (for error reporting and :save).
    pub source_history: Vec<String>,
    /// Input counter (for generating unique function names: __repl_0, __repl_1, ...).
    pub input_counter: u32,
    /// JIT code generator (persistent Cranelift module).
    pub jit: JITCodeGen,
    /// The heap-allocated environment holding live variable values.
    pub env: ReplEnv,
    /// Function definitions declared in prior inputs (replayed into every
    /// new program so typecheck/resolve can see them).
    pub func_defs: Vec<FuncDef>,
    /// Let bindings declared in prior inputs (replayed into every wrapper
    /// body so subsequent inputs can reference the bound names).
    pub let_bindings: Vec<LetBinding>,
    /// Type-level items (class, struct, enum, trait, impl, const,
    /// type-alias, newtype, module, use, lib, extern) declared in prior
    /// inputs. Replayed into every new program.
    pub type_items: Vec<TopLevelItem>,
    /// History file path for persistence.
    pub history_path: PathBuf,
}

impl ReplSession {
    /// Create a new REPL session with fresh state.
    pub fn new() -> Result<Self, String> {
        let jit = JITCodeGen::new()?;

        // History path: ~/.config/riven/history
        let history_path = dirs_path().join("history");

        Ok(ReplSession {
            source_history: Vec::new(),
            input_counter: 0,
            jit,
            env: ReplEnv::new(),
            func_defs: Vec::new(),
            let_bindings: Vec::new(),
            type_items: Vec::new(),
            history_path,
        })
    }

    /// Get the next REPL wrapper function name and increment the counter.
    pub fn next_repl_fn_name(&mut self) -> String {
        let name = format!("__repl_{}", self.input_counter);
        self.input_counter += 1;
        name
    }

    /// Record a successfully executed input.
    pub fn record_input(&mut self, input: &str) {
        self.source_history.push(input.to_string());
    }

    /// Reset all state (for :reset command).
    pub fn reset(&mut self) -> Result<(), String> {
        self.source_history.clear();
        self.input_counter = 0;
        self.env.reset();
        self.func_defs.clear();
        self.let_bindings.clear();
        self.type_items.clear();
        // Recreate JIT module (old one can't be reused after reset)
        self.jit = JITCodeGen::new()?;
        Ok(())
    }
}

/// Get the Riven config directory, creating it if needed.
fn dirs_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = PathBuf::from(home).join(".config").join("riven");
    let _ = std::fs::create_dir_all(&path);
    path
}
