//! Runtime function declarations for the LLVM backend.
//!
//! Declares all C runtime functions in the LLVM module so they can be
//! called from generated code.

use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::AddressSpace;

/// Declare all known runtime functions in the LLVM module.
///
/// Uses `add_function` with Import linkage — the linker resolves them
/// against the compiled C runtime object.
pub fn declare_runtime_functions<'ctx>(module: &Module<'ctx>, context: &'ctx Context) {
    let i8_ty = context.i8_type();
    let i64_ty = context.i64_type();
    let f64_ty = context.f64_type();
    let ptr_ty = context.ptr_type(AddressSpace::default());
    let void_ty = context.void_type();

    macro_rules! decl {
        ($name:expr, void, [$($p:expr),*]) => {
            {
                let params: &[BasicMetadataTypeEnum] = &[$($p.into()),*];
                let fn_ty = void_ty.fn_type(params, false);
                if module.get_function($name).is_none() {
                    module.add_function($name, fn_ty, Some(inkwell::module::Linkage::External));
                }
            }
        };
        ($name:expr, $ret:expr, [$($p:expr),*]) => {
            {
                let params: &[BasicMetadataTypeEnum] = &[$($p.into()),*];
                let fn_ty = $ret.fn_type(params, false);
                if module.get_function($name).is_none() {
                    module.add_function($name, fn_ty, Some(inkwell::module::Linkage::External));
                }
            }
        };
    }

    // I/O
    decl!("riven_puts",        void,   [ptr_ty]);
    decl!("riven_print",       void,   [ptr_ty]);
    decl!("riven_eputs",       void,   [ptr_ty]);
    decl!("riven_print_int",   void,   [i64_ty]);
    decl!("riven_print_float", void,   [f64_ty]);

    // Conversions
    decl!("riven_int_to_string",   ptr_ty, [i64_ty]);
    decl!("riven_float_to_string", ptr_ty, [f64_ty]);
    decl!("riven_bool_to_string",  ptr_ty, [i64_ty]);

    // String operations
    decl!("riven_string_concat",   ptr_ty, [ptr_ty, ptr_ty]);
    decl!("riven_string_from",     ptr_ty, [ptr_ty]);
    decl!("riven_string_push_str", ptr_ty, [ptr_ty, ptr_ty]);
    decl!("riven_deref_ptr",       ptr_ty, [ptr_ty]);
    decl!("riven_store_ptr",       void,   [ptr_ty, ptr_ty]);
    decl!("riven_string_len",      i64_ty, [ptr_ty]);
    decl!("riven_string_is_empty", i8_ty,  [ptr_ty]);
    decl!("riven_string_trim",     ptr_ty, [ptr_ty]);
    decl!("riven_string_to_lower", ptr_ty, [ptr_ty]);
    decl!("riven_string_to_upper", ptr_ty, [ptr_ty]);
    decl!("riven_string_chars",    ptr_ty, [ptr_ty]);
    decl!("riven_string_eq",       i64_ty, [ptr_ty, ptr_ty]);
    decl!("riven_string_cmp",      i64_ty, [ptr_ty, ptr_ty]);
    decl!("riven_str_split",       ptr_ty, [ptr_ty, ptr_ty]);
    decl!("riven_str_parse_uint",  ptr_ty, [ptr_ty]);

    // Memory
    decl!("riven_alloc",     ptr_ty, [i64_ty]);
    decl!("riven_dealloc",   void,   [ptr_ty]);
    decl!("riven_realloc",   ptr_ty, [ptr_ty, i64_ty]);

    // Vec operations
    decl!("riven_vec_new",         ptr_ty, []);
    decl!("riven_vec_push",        void,   [ptr_ty, i64_ty]);
    decl!("riven_vec_pop",         ptr_ty, [ptr_ty]);
    decl!("riven_vec_len",         i64_ty, [ptr_ty]);
    decl!("riven_vec_get",         i64_ty, [ptr_ty, i64_ty]);
    decl!("riven_vec_get_opt",     ptr_ty, [ptr_ty, i64_ty]);
    decl!("riven_vec_get_mut",     ptr_ty, [ptr_ty, i64_ty]);
    decl!("riven_vec_get_mut_opt", ptr_ty, [ptr_ty, i64_ty]);
    decl!("riven_vec_is_empty",    i8_ty,  [ptr_ty]);
    decl!("riven_vec_each",        void,   [ptr_ty, ptr_ty]);

    // Hash operations
    decl!("riven_hash_new",          ptr_ty, []);
    decl!("riven_hash_insert",       void,   [ptr_ty, i64_ty, i64_ty]);
    decl!("riven_hash_get",          ptr_ty, [ptr_ty, i64_ty]);
    decl!("riven_hash_contains_key", i8_ty,  [ptr_ty, i64_ty]);
    decl!("riven_hash_len",          i64_ty, [ptr_ty]);
    decl!("riven_hash_is_empty",     i8_ty,  [ptr_ty]);

    // Set operations
    decl!("riven_set_new",       ptr_ty, []);
    decl!("riven_set_insert",    void,   [ptr_ty, i64_ty]);
    decl!("riven_set_contains",  i8_ty,  [ptr_ty, i64_ty]);
    decl!("riven_set_len",       i64_ty, [ptr_ty]);
    decl!("riven_set_is_empty",  i8_ty,  [ptr_ty]);

    // Option/Result helpers
    decl!("riven_option_unwrap_or",       i64_ty, [ptr_ty, i64_ty]);
    decl!("riven_option_expect",          i64_ty, [ptr_ty, ptr_ty]);
    decl!("riven_option_unwrap",          i64_ty, [ptr_ty]);
    decl!("riven_option_is_some",         i8_ty,  [ptr_ty]);
    decl!("riven_option_is_none",         i8_ty,  [ptr_ty]);
    decl!("riven_result_unwrap_or_else",  i64_ty, [ptr_ty, ptr_ty]);
    decl!("riven_result_try_op",          i64_ty, [ptr_ty]);
    decl!("riven_result_expect",          i64_ty, [ptr_ty, ptr_ty]);
    decl!("riven_result_unwrap",          i64_ty, [ptr_ty]);
    decl!("riven_result_is_ok",           i8_ty,  [ptr_ty]);
    decl!("riven_result_is_err",          i8_ty,  [ptr_ty]);
    decl!("riven_result_ok",              ptr_ty, [ptr_ty]);
    decl!("riven_result_err",             ptr_ty, [ptr_ty]);

    // Panic
    decl!("riven_panic", void, [ptr_ty]);
}
