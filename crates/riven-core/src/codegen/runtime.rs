//! Runtime function declarations and name mapping.
//!
//! Documents the C runtime functions available at link time and provides
//! the shared `runtime_name()` mapping used by both Cranelift and LLVM
//! backends.

/// Path to the C runtime source file, relative to the rivenc crate root.
pub const RUNTIME_C_SOURCE: &str = "runtime/runtime.c";

/// All runtime functions that the C runtime provides.
pub const RUNTIME_FUNCTIONS: &[&str] = &[
    "riven_puts",
    "riven_print",
    "riven_eputs",
    "riven_print_int",
    "riven_print_float",
    "riven_int_to_string",
    "riven_float_to_string",
    "riven_bool_to_string",
    "riven_char_to_string",
    "riven_string_concat",
    "riven_string_from",
    "riven_string_to_upper",
    "riven_string_chars",
    "riven_vec_pop",
    "riven_hash_new",
    "riven_hash_insert",
    "riven_hash_get",
    "riven_hash_contains_key",
    "riven_hash_len",
    "riven_hash_is_empty",
    "riven_set_new",
    "riven_set_insert",
    "riven_set_contains",
    "riven_set_len",
    "riven_set_is_empty",
    "riven_alloc",
    "riven_dealloc",
    "riven_realloc",
    "riven_panic",
    "riven_option_expect",
    "riven_option_unwrap",
    "riven_option_is_some",
    "riven_option_is_none",
    "riven_result_expect",
    "riven_result_unwrap",
    "riven_result_is_ok",
    "riven_result_is_err",
    "riven_result_ok",
    "riven_result_err",
];

/// Extract the method name from a mangled `TypeName_method` string.
///
/// Handles generic types like `Vec[T]_push` by finding `]_` as the
/// type/method separator. For simple types, uses the first `_`.
pub fn extract_method_name(mangled: &str) -> &str {
    // Look for `]_` which signals end of generic type params.
    if let Some(pos) = mangled.rfind("]_") {
        &mangled[pos + 2..]
    } else if let Some(pos) = mangled.find('_') {
        &mangled[pos + 1..]
    } else {
        mangled
    }
}

/// Map Riven built-in function names to their runtime C names.
///
/// Handles both top-level functions (puts, eputs) and mangled method
/// names for built-in types (String_from, Vec_push, etc.).
pub fn runtime_name(name: &str) -> &str {
    match name {
        "puts" => "riven_puts",
        "eputs" => "riven_eputs",
        "print" => "riven_print",
        // String methods
        "String_from" => "riven_string_from",
        "String_push_str" => "riven_string_push_str",
        "String_len" => "riven_string_len",
        "String_is_empty" => "riven_string_is_empty",
        "String_trim" => "riven_string_trim",
        "String_to_lower" => "riven_string_to_lower",
        "String_to_upper" => "riven_string_to_upper",
        "String_chars" => "riven_string_chars",
        "String_clone" => "riven_string_from",
        // &str methods
        "&str_split" => "riven_str_split",
        "&str_parse_uint" => "riven_str_parse_uint",
        "&str_len" => "riven_string_len",
        "&str_is_empty" => "riven_string_is_empty",
        "&str_trim" => "riven_string_trim",
        "&str_to_lower" => "riven_string_to_lower",
        "&str_to_upper" => "riven_string_to_upper",
        "&str_chars" => "riven_string_chars",
        "&str_as_str" => "riven_noop_passthrough",
        // super() calls in constructors
        "super" => "riven_noop",
        // yield calls the passed block — treat as passthrough for v1
        "yield" => "riven_noop_passthrough",
        _ => {
            // Handle generic/complex mangled names by prefix matching.
            let method = extract_method_name(name);

            // Function type call: Fn(...)_call — closure invocation.
            if name.starts_with("Fn(") || name.starts_with("Fn[") {
                return "riven_noop_passthrough";
            }

            // VecIter_, VecIntoIter_, SplitIter_ methods (must check BEFORE Vec[...])
            if name.starts_with("VecIter") || name.starts_with("VecIntoIter")
                || name.starts_with("SplitIter")
            {
                return match method {
                    "filter" => "riven_noop_passthrough",
                    "find" => "riven_noop_return_null",
                    "position" => "riven_noop_return_null",
                    "partition" => "riven_noop_passthrough",
                    "enumerate" => "riven_noop_passthrough",
                    "to_vec" => "riven_noop_passthrough",
                    _ => name,
                };
            }

            // Hash[...] methods (must come before Vec to avoid the
            // accidental match "HashMap" or similar — here we pair
            // Hash explicitly).
            if name.starts_with("Hash[") || name.starts_with("Hash_") {
                return match method {
                    "new" => "riven_hash_new",
                    "insert" => "riven_hash_insert",
                    "get" => "riven_hash_get",
                    "contains_key" => "riven_hash_contains_key",
                    "len" => "riven_hash_len",
                    "is_empty" => "riven_hash_is_empty",
                    _ => name,
                };
            }

            // Set[...] methods
            if name.starts_with("Set[") || name.starts_with("Set_") {
                return match method {
                    "new" => "riven_set_new",
                    "insert" => "riven_set_insert",
                    "contains" => "riven_set_contains",
                    "len" => "riven_set_len",
                    "is_empty" => "riven_set_is_empty",
                    _ => name,
                };
            }

            // Vec[...] methods
            if name.starts_with("Vec") {
                return match method {
                    "new" => "riven_vec_new",
                    "push" => "riven_vec_push",
                    "pop" => "riven_vec_pop",
                    "len" => "riven_vec_len",
                    "get" | "get_mut" => "riven_vec_get_opt",
                    "is_empty" => "riven_vec_is_empty",
                    "each" => "riven_vec_each",
                    "iter" => "riven_noop_passthrough",
                    "into_iter" => "riven_noop_passthrough",
                    "to_vec" => "riven_noop_passthrough",
                    _ => name,
                };
            }

            // Option[...] methods
            if name.starts_with("Option") || name.contains("Option[") {
                return match method {
                    "unwrap_or" => "riven_option_unwrap_or",
                    "expect!" => "riven_option_expect",
                    "unwrap!" => "riven_option_unwrap",
                    "is_some" => "riven_option_is_some",
                    "is_none" => "riven_option_is_none",
                    "map" => "riven_noop_passthrough",
                    _ => "riven_noop_passthrough",
                };
            }

            // Result[...] methods
            if name.starts_with("Result") || name.contains("Result[") {
                return match method {
                    "unwrap_or_else" => "riven_result_unwrap_or_else",
                    "try_op" => "riven_result_try_op",
                    "expect!" => "riven_result_expect",
                    "unwrap!" => "riven_result_unwrap",
                    "is_ok" => "riven_result_is_ok",
                    "is_err" => "riven_result_is_err",
                    "ok" => "riven_result_ok",
                    "err" => "riven_result_err",
                    "map_err" => "riven_noop_passthrough",
                    "ok_or" => "riven_noop_passthrough",
                    _ => name,
                };
            }

            // Inferred type method calls (?T..._method)
            if name.starts_with("?T") || name.starts_with("?") {
                return match method {
                    // Result/Option combinators
                    "try_op" => "riven_result_try_op",
                    "ok_or" => "riven_noop_passthrough",
                    "map_err" | "map" => "riven_noop_passthrough",
                    "unwrap_or" => "riven_option_unwrap_or",
                    "unwrap_or_else" => "riven_result_unwrap_or_else",
                    // String operations
                    "clone" => "riven_string_from",
                    "to_string" | "to_s" => "riven_noop_passthrough",
                    "from" => "riven_string_from",
                    "push_str" => "riven_string_push_str",
                    "trim" => "riven_string_trim",
                    "to_lower" => "riven_string_to_lower",
                    // Vec/collection operations
                    "len" => "riven_vec_len",
                    "is_empty" => "riven_vec_is_empty",
                    "push" => "riven_vec_push",
                    "pop" => "riven_vec_pop",
                    "get" | "get_mut" => "riven_vec_get_opt",
                    "iter" | "into_iter" | "to_vec" => "riven_noop_passthrough",
                    "each" => "riven_vec_each",
                    "filter" | "enumerate" | "partition" => "riven_noop_passthrough",
                    "find" | "position" => "riven_noop_return_null",
                    // User-defined methods — resolve at link time via suffix matching.
                    "message" | "summary" | "is_actionable"
                    | "is_done" | "weight" | "id" | "title_ref"
                    | "priority_ref" | "deadline_ref" | "serialize"
                    | "is_overdue" => name,
                    "to_display" => name,
                    // Mutation methods
                    "assign" | "complete" | "cancel" => name,
                    // Default: treat as noop passthrough (safe fallback).
                    _ => "riven_noop_passthrough",
                };
            }

            // Generic type parameter methods (e.g., "T_assign", "E_message")
            if let Some(pos) = name.find('_') {
                let prefix = &name[..pos];
                if prefix.len() <= 2
                    && !prefix.is_empty()
                    && prefix.chars().all(|c| c.is_ascii_uppercase())
                {
                    return name;
                }
            }

            name
        }
    }
}
