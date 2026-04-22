//! Result display formatting for the REPL.
//!
//! Formats values with their types: `=> value : Type`
//! Uses ANSI colors for terminal output.


use riven_core::hir::types::Ty;
use riven_core::parser::ast::{FuncDef, Program, TopLevelItem};
use riven_core::typeck;

// ANSI color codes
const GREEN: &str = "\x1b[32m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

/// Format and display an expression result with its type.
///
/// Returns the formatted string. Unit results return None (suppressed).
pub fn format_result(raw_value: i64, ty: &Ty) -> Option<String> {
    match ty {
        Ty::Unit => None,
        _ => {
            let value_str = format_value(raw_value, ty);
            let type_str = format_type(ty);
            Some(format!(
                "{GREEN}=>{RESET} {value_str} {DIM}: {type_str}{RESET}"
            ))
        }
    }
}

/// Format a raw value according to its type.
pub fn format_value(raw: i64, ty: &Ty) -> String {
    match ty {
        Ty::Int | Ty::Int64 | Ty::ISize => format!("{}", raw),
        Ty::Int8 => format!("{}", raw as i8),
        Ty::Int16 => format!("{}", raw as i16),
        Ty::Int32 => format!("{}", raw as i32),
        Ty::UInt | Ty::UInt64 | Ty::USize => format!("{}", raw as u64),
        Ty::UInt8 => format!("{}", raw as u8),
        Ty::UInt16 => format!("{}", raw as u16),
        Ty::UInt32 => format!("{}", raw as u32),
        Ty::Float | Ty::Float64 => {
            let f = f64::from_bits(raw as u64);
            format!("{}", f)
        }
        Ty::Float32 => {
            let f = f32::from_bits(raw as u32);
            format!("{}", f)
        }
        Ty::Bool => {
            if raw != 0 { "true".to_string() } else { "false".to_string() }
        }
        Ty::Char => {
            if let Some(c) = char::from_u32(raw as u32) {
                format!("'{}'", c)
            } else {
                format!("'\\u{{{:x}}}'", raw)
            }
        }
        Ty::String | Ty::Str => {
            if raw == 0 {
                "\"\"".to_string()
            } else {
                // Safety: The C runtime returns null-terminated strings that
                // live in malloc'd memory. Copy immediately to a Rust String
                // to avoid lifetime concerns with C-managed memory.
                let ptr = raw as *const std::ffi::c_char;
                let s = unsafe { std::ffi::CStr::from_ptr(ptr) };
                match s.to_str() {
                    Ok(s) => format!("\"{}\"", s),
                    Err(_) => format!("<invalid string at 0x{:x}>", raw),
                }
            }
        }
        _ => {
            // For composite types, just show the raw pointer/value
            if raw == 0 {
                "nil".to_string()
            } else {
                format!("<{} at 0x{:x}>", format_type(ty), raw as u64)
            }
        }
    }
}

/// Format a Ty as a human-readable type string.
pub fn format_type(ty: &Ty) -> String {
    match ty {
        Ty::Int => "Int".to_string(),
        Ty::Int8 => "Int8".to_string(),
        Ty::Int16 => "Int16".to_string(),
        Ty::Int32 => "Int32".to_string(),
        Ty::Int64 => "Int64".to_string(),
        Ty::UInt => "UInt".to_string(),
        Ty::UInt8 => "UInt8".to_string(),
        Ty::UInt16 => "UInt16".to_string(),
        Ty::UInt32 => "UInt32".to_string(),
        Ty::UInt64 => "UInt64".to_string(),
        Ty::ISize => "ISize".to_string(),
        Ty::USize => "USize".to_string(),
        Ty::Float => "Float".to_string(),
        Ty::Float32 => "Float32".to_string(),
        Ty::Float64 => "Float64".to_string(),
        Ty::Bool => "Bool".to_string(),
        Ty::Char => "Char".to_string(),
        Ty::Unit => "Unit".to_string(),
        Ty::Never => "Never".to_string(),
        Ty::String => "String".to_string(),
        Ty::Str => "&str".to_string(),
        Ty::Vec(inner) => format!("Vec[{}]", format_type(inner)),
        Ty::HashMap(k, v) => format!("HashMap[{}, {}]", format_type(k), format_type(v)),
        Ty::Set(inner) => format!("Set[{}]", format_type(inner)),
        Ty::Option(inner) => format!("Option[{}]", format_type(inner)),
        Ty::Result(ok, err) => format!("Result[{}, {}]", format_type(ok), format_type(err)),
        Ty::Ref(inner) => format!("&{}", format_type(inner)),
        Ty::RefMut(inner) => format!("&mut {}", format_type(inner)),
        Ty::Tuple(elems) => {
            let parts: Vec<String> = elems.iter().map(|t| format_type(t)).collect();
            format!("({})", parts.join(", "))
        }
        Ty::Array(inner, size) => format!("[{}; {}]", format_type(inner), size),
        Ty::Fn { params, ret } => {
            let parts: Vec<String> = params.iter().map(|t| format_type(t)).collect();
            format!("Fn({}) -> {}", parts.join(", "), format_type(ret))
        }
        Ty::Class { name, .. } => name.clone(),
        Ty::Struct { name, .. } => name.clone(),
        Ty::Enum { name, .. } => name.clone(),
        _ => format!("{:?}", ty),
    }
}

/// Format a function's type with parameter names — used by `:type foo`
/// when `foo` is a known user-defined function. `Ty::Fn` alone carries
/// only anonymous parameter types, so we fall back to type-checking the
/// def (together with any other known defs it may reference) and
/// stringifying the resolved parameter list.
pub fn format_fn_type_for_def(target: &FuncDef, all_defs: &[FuncDef]) -> String {
    // Build a program containing all accumulated defs so the target can
    // reference its siblings during type checking.
    let items: Vec<TopLevelItem> = all_defs.iter()
        .cloned()
        .map(TopLevelItem::Function)
        .collect();
    let program = Program {
        items,
        span: target.span.clone(),
    };
    let result = typeck::type_check(&program);

    let resolved = result.program.items.iter()
        .filter_map(|item| {
            if let riven_core::hir::nodes::HirItem::Function(f) = item {
                if f.name == target.name {
                    let params: Vec<(String, Ty)> = f.params.iter()
                        .map(|p| (p.name.clone(), p.ty.clone()))
                        .collect();
                    return Some((params, f.return_ty.clone()));
                }
            }
            None
        })
        .next();

    let (params, return_ty) = resolved.unwrap_or_else(|| (Vec::new(), Ty::Unit));

    let param_strs: Vec<String> = params.iter()
        .map(|(n, t)| format!("{}: {}", n, format_type(t)))
        .collect();
    format!("Fn({}) -> {}", param_strs.join(", "), format_type(&return_ty))
}

/// Format a function signature for display after definition.
pub fn format_fn_signature(name: &str, params: &[(String, Ty)], return_ty: &Ty) -> String {
    let param_strs: Vec<String> = params
        .iter()
        .map(|(n, t)| format!("{}: {}", n, format_type(t)))
        .collect();
    format!(
        "{GREEN}=>{RESET} {name} {DIM}: Fn({}) -> {}{RESET}",
        param_strs.join(", "),
        format_type(return_ty)
    )
}

/// Format a REPL error message (compact, 2-4 lines).
pub fn format_error(message: &str) -> String {
    format!("{RED}{BOLD}Error:{RESET} {RED}{message}{RESET}")
}

/// Format a REPL error with a hint.
pub fn format_error_with_hint(message: &str, hint: &str) -> String {
    format!(
        "{RED}{BOLD}Error:{RESET} {RED}{message}{RESET}\n  {CYAN}Hint:{RESET} {hint}"
    )
}

/// Format a warning message.
pub fn format_warning(message: &str) -> String {
    format!("{YELLOW}Warning:{RESET} {message}")
}
