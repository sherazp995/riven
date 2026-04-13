use riven_core::hir::types::Ty;
use riven_core::parser::ast::Visibility;
use riven_core::resolve::symbols::{DefKind, Definition};

use crate::analysis::AnalysisResult;
use crate::node_finder::{node_at_position, NodeAtPosition};

pub struct HoverInfo {
    pub content: String,
    pub range: lsp_types::Range,
}

pub fn hover_at(result: &AnalysisResult, position: lsp_types::Position) -> Option<HoverInfo> {
    let program = result.program.as_ref()?;
    let symbols = result.symbols.as_ref()?;
    let byte_offset = result.line_index.byte_offset_of(position);
    let node = node_at_position(program, byte_offset)?;

    match node {
        NodeAtPosition::VarRef(def_id, span) => {
            let def = symbols.get(def_id)?;
            let ty = symbols.def_ty(def_id)?;
            Some(HoverInfo {
                content: format_variable_hover(def, &ty),
                range: result.line_index.span_to_range(&span),
            })
        }
        NodeAtPosition::FnCall { callee, span } => {
            let def = symbols.get(callee)?;
            Some(HoverInfo {
                content: format_function_hover(def),
                range: result.line_index.span_to_range(&span),
            })
        }
        NodeAtPosition::MethodCall { method, span } => {
            let def = symbols.get(method)?;
            Some(HoverInfo {
                content: format_function_hover(def),
                range: result.line_index.span_to_range(&span),
            })
        }
        NodeAtPosition::FieldAccess {
            object_ty,
            field_name,
            span,
        } => Some(HoverInfo {
            content: format!("```riven\n(field) {}: {}\n```", field_name, object_ty),
            range: result.line_index.span_to_range(&span),
        }),
        NodeAtPosition::TypeRef { name, span } => Some(HoverInfo {
            content: format!("```riven\ntype {}\n```", name),
            range: result.line_index.span_to_range(&span),
        }),
        NodeAtPosition::Definition(def_id, span) => {
            let def = symbols.get(def_id)?;
            Some(HoverInfo {
                content: format_definition_hover(def),
                range: result.line_index.span_to_range(&span),
            })
        }
    }
}

fn format_variable_hover(def: &Definition, ty: &Ty) -> String {
    let prefix = match &def.kind {
        DefKind::Variable { mutable: true, .. } => "let mut",
        DefKind::Variable { mutable: false, .. } => "let",
        DefKind::Param { .. } => "param",
        DefKind::SelfValue { .. } => "self",
        _ => "",
    };
    format!("```riven\n{} {}: {}\n```", prefix, def.name, ty)
}

fn format_function_hover(def: &Definition) -> String {
    match &def.kind {
        DefKind::Function { signature } | DefKind::Method { signature, .. } => {
            let params = signature
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.ty))
                .collect::<Vec<_>>()
                .join(", ");
            let vis = match def.visibility {
                Visibility::Public => "pub ",
                Visibility::Protected => "protected ",
                Visibility::Private => "",
            };
            format!(
                "```riven\n{}def {}({}) -> {}\n```",
                vis, def.name, params, signature.return_ty
            )
        }
        _ => format!("```riven\n{}\n```", def.name),
    }
}

fn format_definition_hover(def: &Definition) -> String {
    match &def.kind {
        DefKind::Variable { ty, mutable, .. } => {
            let prefix = if *mutable { "let mut" } else { "let" };
            format!("```riven\n{} {}: {}\n```", prefix, def.name, ty)
        }
        DefKind::Param { ty, .. } => {
            format!("```riven\nparam {}: {}\n```", def.name, ty)
        }
        DefKind::Function { .. } | DefKind::Method { .. } => format_function_hover(def),
        DefKind::Class { .. } => {
            format!("```riven\nclass {}\n```", def.name)
        }
        DefKind::Struct { .. } => {
            format!("```riven\nstruct {}\n```", def.name)
        }
        DefKind::Enum { .. } => {
            format!("```riven\nenum {}\n```", def.name)
        }
        DefKind::Trait { .. } => {
            format!("```riven\ntrait {}\n```", def.name)
        }
        DefKind::Field { ty, .. } => {
            format!("```riven\n(field) {}: {}\n```", def.name, ty)
        }
        DefKind::EnumVariant { .. } => {
            format!("```riven\n(variant) {}\n```", def.name)
        }
        DefKind::TypeAlias { target } => {
            format!("```riven\ntype {} = {}\n```", def.name, target)
        }
        DefKind::Newtype { inner } => {
            format!("```riven\nnewtype {} = {}\n```", def.name, inner)
        }
        DefKind::Const { ty } => {
            format!("```riven\nconst {}: {}\n```", def.name, ty)
        }
        DefKind::SelfValue { ty } => {
            format!("```riven\nself: {}\n```", ty)
        }
        DefKind::TypeParam { bounds } => {
            if bounds.is_empty() {
                format!("```riven\ntype param {}\n```", def.name)
            } else {
                let bounds_str = bounds
                    .iter()
                    .map(|b| format!("{}", b))
                    .collect::<Vec<_>>()
                    .join(" + ");
                format!("```riven\ntype param {}: {}\n```", def.name, bounds_str)
            }
        }
        DefKind::Module { .. } => {
            format!("```riven\nmodule {}\n```", def.name)
        }
    }
}
