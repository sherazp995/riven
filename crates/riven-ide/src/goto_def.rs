use crate::analysis::AnalysisResult;
use crate::node_finder::{node_at_position, NodeAtPosition};

/// Find the definition location for the symbol at the given position.
///
/// Returns a Location with a placeholder URI — the LSP server replaces
/// it with the actual document URI (all definitions are single-file in Phase 1).
pub fn goto_definition(
    result: &AnalysisResult,
    position: lsp_types::Position,
) -> Option<lsp_types::Location> {
    let program = result.program.as_ref()?;
    let symbols = result.symbols.as_ref()?;
    let byte_offset = result.line_index.byte_offset_of(position);
    let node = node_at_position(program, byte_offset)?;

    let def_id = match node {
        NodeAtPosition::VarRef(id, _) => id,
        NodeAtPosition::FnCall { callee, .. } => callee,
        NodeAtPosition::MethodCall { method, .. } => method,
        NodeAtPosition::TypeRef { .. } => return None,
        NodeAtPosition::FieldAccess { .. } => return None,
        NodeAtPosition::Definition(_, _) => return None, // already at definition
    };

    let definition = symbols.get(def_id)?;

    // Skip built-in definitions (synthetic span with line 0)
    if definition.span.line == 0 && definition.span.start == 0 && definition.span.end == 0 {
        return None;
    }

    let range = result.line_index.span_to_range(&definition.span);

    Some(lsp_types::Location {
        uri: lsp_types::Url::parse("file:///placeholder").unwrap(),
        range,
    })
}
