use riven_core::borrow_check;
use riven_core::borrow_check::errors::BorrowError;
use riven_core::diagnostics::{Diagnostic, DiagnosticLevel};
use riven_core::hir::context::TypeContext;
use riven_core::hir::nodes::HirProgram;
use riven_core::lexer::Lexer;
use riven_core::parser::Parser;
use riven_core::resolve::symbols::SymbolTable;
use riven_core::typeck;

use crate::line_index::LineIndex;

pub struct AnalysisResult {
    pub program: Option<HirProgram>,
    pub symbols: Option<SymbolTable>,
    pub type_context: Option<TypeContext>,
    pub diagnostics: Vec<Diagnostic>,
    pub borrow_errors: Vec<BorrowError>,
    pub source: String,
    pub line_index: LineIndex,
}

impl AnalysisResult {
    fn error_only(source: &str, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            program: None,
            symbols: None,
            type_context: None,
            diagnostics,
            borrow_errors: Vec::new(),
            source: source.to_string(),
            line_index: LineIndex::new(source),
        }
    }
}

/// Run the full analysis pipeline on a source string.
///
/// The pipeline is error-resilient: each phase gates the next.
/// - Lexer errors stop the pipeline (no tokens to parse)
/// - Parser errors stop the pipeline (no AST to type-check)
/// - Type errors gate the borrow checker
pub fn analyze(source: &str) -> AnalysisResult {
    let mut all_diagnostics = Vec::new();

    // Phase 1: Lex
    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(lex_errors) => {
            return AnalysisResult::error_only(source, lex_errors);
        }
    };

    // Phase 2: Parse
    let mut parser = Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(parse_errors) => {
            return AnalysisResult::error_only(source, parse_errors);
        }
    };

    // Phase 3: Type check (always produces a result, even with errors)
    let type_result = typeck::type_check(&ast);
    let has_type_errors = type_result
        .diagnostics
        .iter()
        .any(|d| d.level == DiagnosticLevel::Error);
    all_diagnostics.extend(type_result.diagnostics);

    // Phase 4: Borrow check (only if no type errors)
    let borrow_errors = if has_type_errors {
        Vec::new()
    } else {
        borrow_check::borrow_check(&type_result.program, &type_result.symbols)
    };

    AnalysisResult {
        program: Some(type_result.program),
        symbols: Some(type_result.symbols),
        type_context: Some(type_result.type_context),
        diagnostics: all_diagnostics,
        borrow_errors,
        source: source.to_string(),
        line_index: LineIndex::new(source),
    }
}
