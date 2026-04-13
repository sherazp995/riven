use lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location,
    NumberOrString, Url,
};
use riven_core::borrow_check::errors::BorrowError;
use riven_core::diagnostics::{Diagnostic, DiagnosticLevel};

use crate::analysis::AnalysisResult;
use crate::line_index::LineIndex;

pub fn to_lsp_diagnostic(diag: &Diagnostic, line_index: &LineIndex) -> LspDiagnostic {
    LspDiagnostic {
        range: line_index.span_to_range(&diag.span),
        severity: Some(match diag.level {
            DiagnosticLevel::Error => DiagnosticSeverity::ERROR,
            DiagnosticLevel::Warning => DiagnosticSeverity::WARNING,
            DiagnosticLevel::Help => DiagnosticSeverity::HINT,
        }),
        code: diag
            .code
            .as_ref()
            .map(|c| NumberOrString::String(c.clone())),
        source: Some("rivenc".to_string()),
        message: diag.message.clone(),
        related_information: None,
        ..Default::default()
    }
}

pub fn borrow_error_to_lsp(
    err: &BorrowError,
    line_index: &LineIndex,
    uri: &Url,
) -> LspDiagnostic {
    let related: Vec<DiagnosticRelatedInformation> = err
        .secondary
        .iter()
        .map(|label| DiagnosticRelatedInformation {
            location: Location {
                uri: uri.clone(),
                range: line_index.span_to_range(&label.span),
            },
            message: label.label.clone(),
        })
        .collect();

    LspDiagnostic {
        range: line_index.span_to_range(&err.primary.span),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(err.code.code_str().to_string())),
        source: Some("rivenc".to_string()),
        message: format!("{}: {}", err.code.title(), err.primary.label),
        related_information: if related.is_empty() {
            None
        } else {
            Some(related)
        },
        ..Default::default()
    }
}

/// Convert all diagnostics from an AnalysisResult into LSP diagnostics.
pub fn collect_diagnostics(result: &AnalysisResult, uri: &Url) -> Vec<LspDiagnostic> {
    let mut lsp_diagnostics: Vec<LspDiagnostic> = result
        .diagnostics
        .iter()
        .map(|d| to_lsp_diagnostic(d, &result.line_index))
        .collect();

    for err in &result.borrow_errors {
        lsp_diagnostics.push(borrow_error_to_lsp(err, &result.line_index, uri));
    }

    lsp_diagnostics
}
