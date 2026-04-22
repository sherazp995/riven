//! Cranelift JIT code generation for the Riven REPL.
//!
//! Wraps `JITModule` for in-process compilation and execution.
//! Separate from the batch `CodeGen` (which uses `ObjectModule`)
//! to avoid generification complexity.

use std::collections::HashMap;

use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::types::{self, Type};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Signature};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, FuncId, Linkage, Module};

use riven_core::hir::types::Ty;
use riven_core::mir::nodes::*;
use riven_core::parser::ast::BinOp;
use riven_core::codegen::runtime::{extract_method_name, runtime_name};

// ── C runtime function declarations ────────────────────────────────
// These are linked into the REPL binary and registered as JIT symbols.

extern "C" {
    fn riven_puts(s: *const i8);
    fn riven_print(s: *const i8);
    fn riven_eputs(s: *const i8);
    fn riven_print_int(n: i64);
    fn riven_print_float(f: f64);
    fn riven_int_to_string(n: i64) -> *const i8;
    fn riven_float_to_string(f: f64) -> *const i8;
    fn riven_bool_to_string(b: i64) -> *const i8;
    fn riven_string_concat(a: *const i8, b: *const i8) -> *const i8;
    fn riven_string_from(s: *const i8) -> *const i8;
    fn riven_string_push_str(s: *const i8, t: *const i8) -> *const i8;
    fn riven_string_len(s: *const i8) -> i64;
    fn riven_string_is_empty(s: *const i8) -> i8;
    fn riven_string_trim(s: *const i8) -> *const i8;
    fn riven_string_to_lower(s: *const i8) -> *const i8;
    fn riven_string_eq(a: *const i8, b: *const i8) -> i64;
    fn riven_string_cmp(a: *const i8, b: *const i8) -> i64;
    fn riven_str_split(s: *const i8, d: *const i8) -> *const i8;
    fn riven_str_parse_uint(s: *const i8) -> i64;
    fn riven_alloc(size: i64) -> *mut u8;
    fn riven_dealloc(ptr: *mut u8);
    fn riven_realloc(ptr: *mut u8, new_size: i64) -> *mut u8;
    fn riven_panic(msg: *const i8);
    fn riven_vec_new() -> *mut u8;
    fn riven_vec_push(v: *mut u8, item: i64);
    fn riven_vec_len(v: *mut u8) -> i64;
    fn riven_vec_get(v: *mut u8, idx: i64) -> i64;
    fn riven_vec_get_opt(v: *mut u8, idx: i64) -> i64;
    fn riven_vec_get_mut(v: *mut u8, idx: i64) -> i64;
    fn riven_vec_get_mut_opt(v: *mut u8, idx: i64) -> i64;
    fn riven_vec_is_empty(v: *mut u8) -> i8;
    fn riven_vec_each(v: *mut u8, cb: *const u8);
    fn riven_option_unwrap_or(opt: *mut u8, default: i64) -> i64;
    fn riven_result_unwrap_or_else(result: *mut u8, handler: *const u8) -> i64;
    fn riven_result_try_op(result: *mut u8) -> i64;
    fn riven_noop_passthrough(val: i64) -> i64;
    fn riven_noop_return_null() -> i64;
    fn riven_noop();
}

/// Cranelift JIT code generation engine for the REPL.
pub struct JITCodeGen {
    module: JITModule,
    ctx: Context,
    builder_ctx: FunctionBuilderContext,
    string_data: HashMap<String, cranelift_module::DataId>,
    string_counter: u32,
    declared_fns: HashMap<String, FuncId>,
}

impl JITCodeGen {
    /// Create a new JIT code generator targeting the host machine.
    ///
    /// Key difference from batch `CodeGen`: `is_pic = false` since JIT code
    /// runs at known absolute addresses in process memory.
    pub fn new() -> Result<Self, String> {
        let mut flag_builder = settings::builder();
        flag_builder
            .set("opt_level", "none")
            .map_err(|e| format!("Failed to set opt_level: {}", e))?;
        // JIT code runs at absolute addresses — NOT position-independent
        flag_builder
            .set("is_pic", "false")
            .map_err(|e| format!("Failed to set is_pic: {}", e))?;

        let isa_builder = cranelift_native::builder()
            .map_err(|e| format!("Failed to create native ISA builder: {}", e))?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| format!("Failed to finish ISA: {}", e))?;

        let mut jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Register all C runtime functions as JIT symbols
        register_runtime_symbols(&mut jit_builder);

        let module = JITModule::new(jit_builder);
        let ctx = module.make_context();

        Ok(JITCodeGen {
            module,
            ctx,
            builder_ctx: FunctionBuilderContext::new(),
            string_data: HashMap::new(),
            string_counter: 0,
            declared_fns: HashMap::new(),
        })
    }

    /// Compile a single REPL input (wrapped as `__repl_N` function) and return
    /// a callable function pointer.
    pub fn compile_repl_input(
        &mut self,
        mir_function: &MirFunction,
    ) -> Result<*const u8, String> {
        // Declare
        let sig = build_jit_signature(&self.module, mir_function);
        let func_id = self
            .module
            .declare_function(&mir_function.name, Linkage::Export, &sig)
            .map_err(|e| format!("Failed to declare REPL function '{}': {}", mir_function.name, e))?;

        self.declared_fns.insert(mir_function.name.clone(), func_id);

        // Define — on failure, drop the declared_fns entry so a retry
        // with a fresh wrapper name isn't blocked by a dangling symbol.
        if let Err(e) = self.compile_function_inner(mir_function, func_id) {
            self.declared_fns.remove(&mir_function.name);
            return Err(e);
        }

        // Finalize and get pointer
        self.module.finalize_definitions()
            .map_err(|e| format!("Failed to finalize: {}", e))?;

        let code_ptr = self.module.get_finalized_function(func_id);
        Ok(code_ptr)
    }

    /// Compile a user-defined function and register it in the JIT module.
    pub fn compile_function(
        &mut self,
        mir_function: &MirFunction,
    ) -> Result<(), String> {
        let sig = build_jit_signature(&self.module, mir_function);
        let linkage = Linkage::Export;

        let func_id = self
            .module
            .declare_function(&mir_function.name, linkage, &sig)
            .map_err(|e| format!("Failed to declare function '{}': {}", mir_function.name, e))?;

        self.declared_fns.insert(mir_function.name.clone(), func_id);
        self.compile_function_inner(mir_function, func_id)?;

        self.module.finalize_definitions()
            .map_err(|e| format!("Failed to finalize: {}", e))?;

        Ok(())
    }

    /// Check if a function name is already declared in the JIT module.
    pub fn is_declared(&self, name: &str) -> bool {
        self.declared_fns.contains_key(name)
    }

    /// Internal: translate MIR to Cranelift IR and define the function.
    fn compile_function_inner(
        &mut self,
        func: &MirFunction,
        func_id: FuncId,
    ) -> Result<(), String> {
        let sig = build_jit_signature(&self.module, func);
        self.ctx.func.signature = sig;

        {
            let mut builder =
                FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_ctx);

            let mut env = JITTranslationEnv {
                module: &mut self.module,
                declared_fns: &mut self.declared_fns,
                string_data: &mut self.string_data,
                string_counter: &mut self.string_counter,
            };

            // Map MIR blocks → Cranelift blocks
            let mut block_map: Vec<cranelift_codegen::ir::Block> =
                Vec::with_capacity(func.blocks.len());
            for _ in &func.blocks {
                block_map.push(builder.create_block());
            }

            // Declare Cranelift Variables for all locals
            let mut var_map: HashMap<LocalId, Variable> = HashMap::new();
            for local in &func.locals {
                let cl_ty = ty_to_cranelift(&local.ty).unwrap_or(types::I64);
                let var = builder.declare_var(cl_ty);
                var_map.insert(local.id, var);
            }

            // Set up entry block
            let entry_cl_block = block_map[func.entry_block];
            builder.switch_to_block(entry_cl_block);
            builder.append_block_params_for_function_params(entry_cl_block);

            // Bind function parameters to their local variables
            let params_vals = builder.block_params(entry_cl_block).to_vec();
            for (i, &param_id) in func.params.iter().enumerate() {
                if i < params_vals.len() {
                    if let Some(&var) = var_map.get(&param_id) {
                        builder.def_var(var, params_vals[i]);
                    }
                }
            }

            // Translate each block
            for (mir_idx, mir_block) in func.blocks.iter().enumerate() {
                let cl_block = block_map[mir_idx];

                if mir_idx != func.entry_block {
                    builder.switch_to_block(cl_block);
                }

                for inst in &mir_block.instructions {
                    if let Err(e) = translate_instruction(
                        inst, func, &var_map, &block_map, &mut builder, &mut env,
                    ) {
                        return Err(format!(
                            "Error in function '{}', block {}: {}",
                            func.name, mir_idx, e
                        ));
                    }
                }

                translate_terminator(
                    &mir_block.terminator, func, &var_map, &block_map,
                    &mut builder, &mut env,
                )?;
            }

            builder.seal_all_blocks();
            builder.finalize();
        }

        let define_result = self.module.define_function(func_id, &mut self.ctx);
        // Always clear the shared context so a failed compilation doesn't
        // leak IR into the next one (without this, a second REPL input
        // sees the prior input's instructions and the verifier complains
        // about stale blocks).
        self.module.clear_context(&mut self.ctx);
        define_result
            .map_err(|e| format!("Failed to define function '{}': {:?}", func.name, e))?;

        Ok(())
    }
}

// ── Runtime symbol registration ────────────────────────────────────

fn register_runtime_symbols(builder: &mut JITBuilder) {
    macro_rules! reg {
        ($builder:expr, $name:ident) => {
            $builder.symbol(stringify!($name), $name as *const u8);
        };
    }

    reg!(builder, riven_puts);
    reg!(builder, riven_print);
    reg!(builder, riven_eputs);
    reg!(builder, riven_print_int);
    reg!(builder, riven_print_float);
    reg!(builder, riven_int_to_string);
    reg!(builder, riven_float_to_string);
    reg!(builder, riven_bool_to_string);
    reg!(builder, riven_string_concat);
    reg!(builder, riven_string_from);
    reg!(builder, riven_string_push_str);
    reg!(builder, riven_string_len);
    reg!(builder, riven_string_is_empty);
    reg!(builder, riven_string_trim);
    reg!(builder, riven_string_to_lower);
    reg!(builder, riven_string_eq);
    reg!(builder, riven_string_cmp);
    reg!(builder, riven_str_split);
    reg!(builder, riven_str_parse_uint);
    reg!(builder, riven_alloc);
    reg!(builder, riven_dealloc);
    reg!(builder, riven_realloc);
    reg!(builder, riven_panic);
    reg!(builder, riven_vec_new);
    reg!(builder, riven_vec_push);
    reg!(builder, riven_vec_len);
    reg!(builder, riven_vec_get);
    reg!(builder, riven_vec_get_opt);
    reg!(builder, riven_vec_get_mut);
    reg!(builder, riven_vec_get_mut_opt);
    reg!(builder, riven_vec_is_empty);
    reg!(builder, riven_vec_each);
    reg!(builder, riven_option_unwrap_or);
    reg!(builder, riven_result_unwrap_or_else);
    reg!(builder, riven_result_try_op);
    reg!(builder, riven_noop_passthrough);
    reg!(builder, riven_noop_return_null);
    reg!(builder, riven_noop);
}

// ── JIT Translation Environment ────────────────────────────────────

struct JITTranslationEnv<'a> {
    module: &'a mut JITModule,
    declared_fns: &'a mut HashMap<String, FuncId>,
    string_data: &'a mut HashMap<String, cranelift_module::DataId>,
    string_counter: &'a mut u32,
}

impl<'a> JITTranslationEnv<'a> {
    fn create_string_data(&mut self, value: &str) -> Result<cranelift_module::DataId, String> {
        if let Some(&data_id) = self.string_data.get(value) {
            return Ok(data_id);
        }

        let name = format!(".str.{}", *self.string_counter);
        *self.string_counter += 1;

        let data_id = self
            .module
            .declare_data(&name, Linkage::Local, false, false)
            .map_err(|e| format!("Failed to declare string data: {}", e))?;

        let mut desc = DataDescription::new();
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        desc.define(bytes.into_boxed_slice());

        self.module
            .define_data(data_id, &desc)
            .map_err(|e| format!("Failed to define string data: {}", e))?;

        self.string_data.insert(value.to_string(), data_id);
        Ok(data_id)
    }

    fn get_or_declare_func(
        &mut self,
        name: &str,
        arg_vals: &[cranelift_codegen::ir::Value],
        has_return: bool,
        builder: &mut FunctionBuilder,
    ) -> Result<cranelift_codegen::ir::FuncRef, String> {
        if let Some(&func_id) = self.declared_fns.get(name) {
            let func_ref = self.module.declare_func_in_func(func_id, builder.func);
            return Ok(func_ref);
        }

        // Try inferred-type method resolution (same as batch codegen)
        if name.starts_with("?") {
            let method = extract_method_name(name);
            let suffix = format!("_{}", method);
            let match_name = self.declared_fns.keys()
                .filter(|k| k.ends_with(&suffix) && !k.starts_with("?"))
                .min_by_key(|k| k.len())
                .cloned();
            if let Some(resolved) = match_name {
                let func_id = self.declared_fns[&resolved];
                let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                return Ok(func_ref);
            }
        }

        // Generic type param resolution
        if !self.declared_fns.contains_key(name) {
            let method = extract_method_name(name);
            let type_prefix = if let Some(pos) = name.find('_') {
                &name[..pos]
            } else {
                ""
            };
            let is_generic_param = type_prefix.len() <= 2
                && type_prefix.chars().all(|c| c.is_ascii_uppercase());
            if is_generic_param && !type_prefix.is_empty() {
                let suffix = format!("_{}", method);
                let match_name = self.declared_fns.keys()
                    .filter(|k| k.ends_with(&suffix) && !k.starts_with("?")
                        && k.len() > suffix.len())
                    .min_by_key(|k| k.len())
                    .cloned();
                if let Some(resolved) = match_name {
                    let func_id = self.declared_fns[&resolved];
                    let func_ref = self.module.declare_func_in_func(func_id, builder.func);
                    return Ok(func_ref);
                }
            }
        }

        // Try known runtime signatures
        if let Some((param_tys, ret_ty)) = runtime_signature(name) {
            return self.declare_runtime_func(name, &param_tys, ret_ty, builder);
        }

        // Fall back: infer signature from call-site
        let call_conv = self.module.isa().default_call_conv();
        let mut sig = Signature::new(call_conv);
        for val in arg_vals {
            let ty = builder.func.dfg.value_type(*val);
            sig.params.push(AbiParam::new(ty));
        }
        if has_return {
            sig.returns.push(AbiParam::new(types::I64));
        }

        let func_id = self
            .module
            .declare_function(name, Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare function '{}': {}", name, e))?;

        self.declared_fns.insert(name.to_string(), func_id);
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        Ok(func_ref)
    }

    fn declare_runtime_func(
        &mut self,
        name: &str,
        params: &[Type],
        ret: Option<Type>,
        builder: &mut FunctionBuilder,
    ) -> Result<cranelift_codegen::ir::FuncRef, String> {
        if let Some(&func_id) = self.declared_fns.get(name) {
            let func_ref = self.module.declare_func_in_func(func_id, builder.func);
            return Ok(func_ref);
        }

        let call_conv = self.module.isa().default_call_conv();
        let mut sig = Signature::new(call_conv);
        for &p in params {
            sig.params.push(AbiParam::new(p));
        }
        if let Some(r) = ret {
            sig.returns.push(AbiParam::new(r));
        }

        let func_id = self
            .module
            .declare_function(name, Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare runtime function '{}': {}", name, e))?;

        self.declared_fns.insert(name.to_string(), func_id);
        let func_ref = self.module.declare_func_in_func(func_id, builder.func);
        Ok(func_ref)
    }
}

// ── Signature building ─────────────────────────────────────────────

fn build_jit_signature(module: &JITModule, func: &MirFunction) -> Signature {
    let call_conv = module.isa().default_call_conv();
    let mut sig = Signature::new(call_conv);

    for &param_id in &func.params {
        let local = &func.locals[param_id as usize];
        if let Some(cl_ty) = ty_to_cranelift(&local.ty) {
            sig.params.push(AbiParam::new(cl_ty));
        }
    }

    if let Some(ret_ty) = ty_to_cranelift(&func.return_ty) {
        sig.returns.push(AbiParam::new(ret_ty));
    }

    sig
}

// ── Instruction translation (mirrors batch codegen) ────────────────

fn translate_instruction(
    inst: &MirInst,
    func: &MirFunction,
    var_map: &HashMap<LocalId, Variable>,
    _block_map: &[cranelift_codegen::ir::Block],
    builder: &mut FunctionBuilder,
    env: &mut JITTranslationEnv,
) -> Result<(), String> {
    match inst {
        MirInst::Assign { dest, value } => {
            let val = gen_value(value, func, var_map, builder)?;
            let dest_ty = func.locals.get(*dest as usize)
                .and_then(|l| ty_to_cranelift(&l.ty))
                .unwrap_or(types::I64);
            let val = coerce_value(val, dest_ty, builder);
            builder.def_var(var_map[dest], val);
        }

        MirInst::BinOp { dest, op, lhs, rhs } => {
            let l = gen_value(lhs, func, var_map, builder)?;
            let r = gen_value(rhs, func, var_map, builder)?;

            // String concatenation: `"a" + "b"` is typed as Ty::String but
            // lowered as a generic BinOp::Add — dispatch to the runtime
            // helper instead of emitting an integer add on the pointers.
            let is_string_concat = matches!(op, BinOp::Add)
                && (is_string_typed_value(lhs, func) || is_string_typed_value(rhs, func));

            let result = if is_string_concat {
                let func_ref = env.declare_runtime_func(
                    "riven_string_concat",
                    &[types::I64, types::I64],
                    Some(types::I64),
                    builder,
                )?;
                let call = builder.ins().call(func_ref, &[l, r]);
                builder.inst_results(call)[0]
            } else {
                let common_ty = builder.func.dfg.value_type(l);
                let r = coerce_value(r, common_ty, builder);
                emit_binop(*op, l, r, builder)
            };

            let dest_ty = func.locals.get(*dest as usize)
                .and_then(|l| ty_to_cranelift(&l.ty))
                .unwrap_or(types::I64);
            let result = coerce_value(result, dest_ty, builder);
            builder.def_var(var_map[dest], result);
        }

        MirInst::Negate { dest, operand } => {
            let val = gen_value(operand, func, var_map, builder)?;
            let result = builder.ins().ineg(val);
            let dest_ty = func.locals.get(*dest as usize)
                .and_then(|l| ty_to_cranelift(&l.ty))
                .unwrap_or(types::I64);
            let result = coerce_value(result, dest_ty, builder);
            builder.def_var(var_map[dest], result);
        }

        MirInst::Not { dest, operand } => {
            let val = gen_value(operand, func, var_map, builder)?;
            let val_ty = builder.func.dfg.value_type(val);
            let one = builder.ins().iconst(val_ty, 1);
            let result = builder.ins().bxor(val, one);
            let dest_ty = func.locals.get(*dest as usize)
                .and_then(|l| ty_to_cranelift(&l.ty))
                .unwrap_or(types::I8);
            let result = coerce_value(result, dest_ty, builder);
            builder.def_var(var_map[dest], result);
        }

        MirInst::Compare { dest, op, lhs, rhs } => {
            let l = gen_value(lhs, func, var_map, builder)?;
            let r = gen_value(rhs, func, var_map, builder)?;

            let is_string_compare = is_string_typed_value(lhs, func)
                || is_string_typed_value(rhs, func);

            let result = if is_string_compare && matches!(op, CmpOp::Eq | CmpOp::NotEq) {
                let func_ref = env.declare_runtime_func(
                    "riven_string_eq", &[types::I64, types::I64], Some(types::I64), builder,
                )?;
                let call = builder.ins().call(func_ref, &[l, r]);
                let eq_result = builder.inst_results(call)[0];
                if matches!(op, CmpOp::NotEq) {
                    let zero = builder.ins().iconst(types::I64, 0);
                    builder.ins().icmp(IntCC::Equal, eq_result, zero)
                } else {
                    builder.ins().ireduce(types::I8, eq_result)
                }
            } else if is_string_compare {
                let func_ref = env.declare_runtime_func(
                    "riven_string_cmp", &[types::I64, types::I64], Some(types::I64), builder,
                )?;
                let call = builder.ins().call(func_ref, &[l, r]);
                let cmp_result = builder.inst_results(call)[0];
                let zero = builder.ins().iconst(types::I64, 0);
                let cc = cmpop_to_intcc(*op);
                builder.ins().icmp(cc, cmp_result, zero)
            } else {
                let common_ty = builder.func.dfg.value_type(l);
                let r = coerce_value(r, common_ty, builder);
                let cc = cmpop_to_intcc(*op);
                builder.ins().icmp(cc, l, r)
            };
            let dest_ty = func.locals.get(*dest as usize)
                .and_then(|l| ty_to_cranelift(&l.ty))
                .unwrap_or(types::I8);
            let result = coerce_value(result, dest_ty, builder);
            builder.def_var(var_map[dest], result);
        }

        MirInst::Call { dest, callee, args } => {
            let mut arg_vals = Vec::with_capacity(args.len());
            for arg in args {
                arg_vals.push(gen_value(arg, func, var_map, builder)?);
            }

            let actual_name = runtime_name(callee);

            match actual_name {
                "riven_noop_passthrough" => {
                    if let Some(dest_id) = dest {
                        let dest_ty = func.locals.get(*dest_id as usize)
                            .and_then(|l| ty_to_cranelift(&l.ty))
                            .unwrap_or(types::I64);
                        let val = if !arg_vals.is_empty() {
                            coerce_value(arg_vals[0], dest_ty, builder)
                        } else {
                            builder.ins().iconst(dest_ty, 0)
                        };
                        builder.def_var(var_map[dest_id], val);
                    }
                }
                "riven_noop_return_null" => {
                    if let Some(dest_id) = dest {
                        let dest_ty = func.locals.get(*dest_id as usize)
                            .and_then(|l| ty_to_cranelift(&l.ty))
                            .unwrap_or(types::I64);
                        let zero = builder.ins().iconst(dest_ty, 0);
                        builder.def_var(var_map[dest_id], zero);
                    }
                }
                "riven_noop" => {
                    if let Some(dest_id) = dest {
                        let dest_ty = func.locals.get(*dest_id as usize)
                            .and_then(|l| ty_to_cranelift(&l.ty))
                            .unwrap_or(types::I64);
                        let zero = builder.ins().iconst(dest_ty, 0);
                        builder.def_var(var_map[dest_id], zero);
                    }
                }
                _ => {
                    let func_ref =
                        env.get_or_declare_func(actual_name, &arg_vals, dest.is_some(), builder)?;
                    let call = builder.ins().call(func_ref, &arg_vals);

                    if let Some(dest_id) = dest {
                        let results = builder.inst_results(call);
                        if !results.is_empty() {
                            let dest_ty = func.locals.get(*dest_id as usize)
                                .and_then(|l| ty_to_cranelift(&l.ty))
                                .unwrap_or(types::I64);
                            let result = coerce_value(results[0], dest_ty, builder);
                            builder.def_var(var_map[dest_id], result);
                        }
                    }
                }
            }
        }

        MirInst::Alloc { dest, ty: alloc_ty, size: precomputed_size } => {
            let size = if *precomputed_size > 0 {
                *precomputed_size as i64
            } else {
                simple_type_size(alloc_ty) as i64
            };
            let size_val = builder.ins().iconst(types::I64, size);
            let func_ref = env.declare_runtime_func(
                "riven_alloc", &[types::I64], Some(types::I64), builder,
            )?;
            let call = builder.ins().call(func_ref, &[size_val]);
            let ptr = builder.inst_results(call)[0];
            builder.def_var(var_map[dest], ptr);
        }

        MirInst::StackAlloc { dest, .. } => {
            let dest_ty = func.locals.get(*dest as usize)
                .and_then(|l| ty_to_cranelift(&l.ty))
                .unwrap_or(types::I64);
            let zero = builder.ins().iconst(dest_ty, 0);
            builder.def_var(var_map[dest], zero);
        }

        MirInst::GetField { dest, base, field_index } => {
            let base_val = builder.use_var(var_map[base]);
            let offset = (*field_index as i64) * 8;
            let addr = builder.ins().iadd_imm(base_val, offset);
            let dest_ty = func.locals.get(*dest as usize)
                .and_then(|l| ty_to_cranelift(&l.ty))
                .unwrap_or(types::I64);
            let loaded = builder.ins().load(dest_ty, MemFlags::new(), addr, 0);
            builder.def_var(var_map[dest], loaded);
        }

        MirInst::SetField { base, field_index, value } => {
            let base_val = builder.use_var(var_map[base]);
            let val = gen_value(value, func, var_map, builder)?;
            let offset = (*field_index as i64) * 8;
            let addr = builder.ins().iadd_imm(base_val, offset);
            builder.ins().store(MemFlags::new(), val, addr, 0);
        }

        MirInst::SetTag { dest, tag } => {
            let ptr = builder.use_var(var_map[dest]);
            let tag_val = builder.ins().iconst(types::I32, *tag as i64);
            builder.ins().store(MemFlags::new(), tag_val, ptr, 0);
        }

        MirInst::GetTag { dest, src } => {
            let ptr = builder.use_var(var_map[src]);
            let tag_val = builder.ins().load(types::I32, MemFlags::new(), ptr, 0);
            builder.def_var(var_map[dest], tag_val);
        }

        MirInst::GetPayload { dest, src, .. } => {
            let ptr = builder.use_var(var_map[src]);
            let payload_ptr = builder.ins().iadd_imm(ptr, 8);
            builder.def_var(var_map[dest], payload_ptr);
        }

        MirInst::Ref { dest, src } | MirInst::RefMut { dest, src } => {
            let val = builder.use_var(var_map[src]);
            builder.def_var(var_map[dest], val);
        }

        MirInst::Copy { dest, src } | MirInst::Move { dest, src } => {
            let val = builder.use_var(var_map[src]);
            builder.def_var(var_map[dest], val);
        }

        MirInst::Drop { local: _ } => {
            // No-op for now (same as batch codegen)
        }

        MirInst::StringLiteral { dest, value } => {
            let gv = env.create_string_data(value)?;
            let ptr = env.module.declare_data_in_func(gv, builder.func);
            let val = builder.ins().global_value(types::I64, ptr);
            builder.def_var(var_map[dest], val);
        }

        MirInst::Nop => {}

        MirInst::FuncAddr { dest, func_name } => {
            let func_ref =
                env.get_or_declare_func(func_name, &[], true, builder)?;
            let addr = builder.ins().func_addr(types::I64, func_ref);
            builder.def_var(var_map[dest], addr);
        }

        MirInst::CallIndirect { dest, callee, args } => {
            let callee_val = builder.use_var(var_map[callee]);
            let mut arg_vals = Vec::with_capacity(args.len());
            for arg in args {
                arg_vals.push(gen_value(arg, func, var_map, builder)?);
            }

            let call_conv = env.module.isa().default_call_conv();
            let mut sig = Signature::new(call_conv);
            for val in &arg_vals {
                let ty = builder.func.dfg.value_type(*val);
                sig.params.push(AbiParam::new(ty));
            }
            if dest.is_some() {
                sig.returns.push(AbiParam::new(types::I64));
            }
            let sig_ref = builder.import_signature(sig);
            let call = builder.ins().call_indirect(sig_ref, callee_val, &arg_vals);

            if let Some(dest_id) = dest {
                let results = builder.inst_results(call);
                if !results.is_empty() {
                    let dest_ty = func.locals.get(*dest_id as usize)
                        .and_then(|l| ty_to_cranelift(&l.ty))
                        .unwrap_or(types::I64);
                    let result = coerce_value(results[0], dest_ty, builder);
                    builder.def_var(var_map[dest_id], result);
                }
            }
        }
    }

    Ok(())
}

fn translate_terminator(
    term: &Terminator,
    func: &MirFunction,
    var_map: &HashMap<LocalId, Variable>,
    block_map: &[cranelift_codegen::ir::Block],
    builder: &mut FunctionBuilder,
    _env: &mut JITTranslationEnv,
) -> Result<(), String> {
    match term {
        Terminator::Return(val) => {
            match val {
                Some(v) => {
                    let ret_val = gen_value(v, func, var_map, builder)?;
                    if let Some(ret_ty) = ty_to_cranelift(&func.return_ty) {
                        let ret_val = coerce_value(ret_val, ret_ty, builder);
                        builder.ins().return_(&[ret_val]);
                    } else {
                        builder.ins().return_(&[]);
                    }
                }
                None => {
                    builder.ins().return_(&[]);
                }
            }
        }

        Terminator::Goto(target) => {
            builder.ins().jump(block_map[*target], &[]);
        }

        Terminator::Branch { cond, then_block, else_block } => {
            let cond_val = gen_value(cond, func, var_map, builder)?;
            builder.ins().brif(
                cond_val,
                block_map[*then_block], &[],
                block_map[*else_block], &[],
            );
        }

        Terminator::Switch { value, targets, otherwise } => {
            let val = gen_value(value, func, var_map, builder)?;
            let mut switch = cranelift_frontend::Switch::new();
            for &(discriminant, block_id) in targets {
                switch.set_entry(discriminant as u128, block_map[block_id]);
            }
            switch.emit(builder, val, block_map[*otherwise]);
        }

        Terminator::Unreachable => {
            builder
                .ins()
                .trap(cranelift_codegen::ir::TrapCode::user(1).unwrap());
        }
    }

    Ok(())
}

// ── Helpers (mirrored from batch codegen) ──────────────────────────

fn gen_value(
    mir_val: &MirValue,
    func: &MirFunction,
    var_map: &HashMap<LocalId, Variable>,
    builder: &mut FunctionBuilder,
) -> Result<cranelift_codegen::ir::Value, String> {
    match mir_val {
        MirValue::Literal(lit) => match lit {
            Literal::Int(n) => Ok(builder.ins().iconst(types::I64, *n)),
            Literal::Float(n) => Ok(builder.ins().f64const(*n)),
            Literal::Bool(b) => Ok(builder.ins().iconst(types::I8, *b as i64)),
            Literal::Char(c) => Ok(builder.ins().iconst(types::I32, *c as i64)),
            Literal::String(_) => Ok(builder.ins().iconst(types::I64, 0)),
        },
        MirValue::Use(local_id) => {
            let var = var_map.get(local_id).ok_or_else(|| {
                format!("Unknown local {} in function '{}'", local_id, func.name)
            })?;
            Ok(builder.use_var(*var))
        }
        MirValue::Unit => Ok(builder.ins().iconst(types::I64, 0)),
    }
}

fn emit_binop(
    op: BinOp,
    lhs: cranelift_codegen::ir::Value,
    rhs: cranelift_codegen::ir::Value,
    builder: &mut FunctionBuilder,
) -> cranelift_codegen::ir::Value {
    let is_float = builder.func.dfg.value_type(lhs).is_float();
    if is_float {
        match op {
            BinOp::Add => builder.ins().fadd(lhs, rhs),
            BinOp::Sub => builder.ins().fsub(lhs, rhs),
            BinOp::Mul => builder.ins().fmul(lhs, rhs),
            BinOp::Div => builder.ins().fdiv(lhs, rhs),
            // Cranelift has no native float remainder; most languages don't
            // expose `%` on floats. Fall back to int rem (will fail verifier
            // if it ever actually runs — surfaced as a compiler error).
            BinOp::Mod => builder.ins().srem(lhs, rhs),
            BinOp::Eq => builder.ins().fcmp(FloatCC::Equal, lhs, rhs),
            BinOp::NotEq => builder.ins().fcmp(FloatCC::NotEqual, lhs, rhs),
            BinOp::Lt => builder.ins().fcmp(FloatCC::LessThan, lhs, rhs),
            BinOp::LtEq => builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs),
            BinOp::Gt => builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs),
            BinOp::GtEq => builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs),
            // Bitwise / logical ops aren't valid on floats; caller shouldn't
            // emit these — keep the integer form to preserve batch-codegen
            // parity.
            BinOp::BitAnd | BinOp::And => builder.ins().band(lhs, rhs),
            BinOp::BitOr | BinOp::Or => builder.ins().bor(lhs, rhs),
            BinOp::BitXor => builder.ins().bxor(lhs, rhs),
            BinOp::Shl => builder.ins().ishl(lhs, rhs),
            BinOp::Shr => builder.ins().sshr(lhs, rhs),
        }
    } else {
        match op {
            BinOp::Add => builder.ins().iadd(lhs, rhs),
            BinOp::Sub => builder.ins().isub(lhs, rhs),
            BinOp::Mul => builder.ins().imul(lhs, rhs),
            BinOp::Div => builder.ins().sdiv(lhs, rhs),
            BinOp::Mod => builder.ins().srem(lhs, rhs),
            BinOp::BitAnd => builder.ins().band(lhs, rhs),
            BinOp::BitOr => builder.ins().bor(lhs, rhs),
            BinOp::BitXor => builder.ins().bxor(lhs, rhs),
            BinOp::Shl => builder.ins().ishl(lhs, rhs),
            BinOp::Shr => builder.ins().sshr(lhs, rhs),
            BinOp::And => builder.ins().band(lhs, rhs),
            BinOp::Or => builder.ins().bor(lhs, rhs),
            BinOp::Eq => builder.ins().icmp(IntCC::Equal, lhs, rhs),
            BinOp::NotEq => builder.ins().icmp(IntCC::NotEqual, lhs, rhs),
            BinOp::Lt => builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs),
            BinOp::LtEq => builder.ins().icmp(IntCC::SignedLessThanOrEqual, lhs, rhs),
            BinOp::Gt => builder.ins().icmp(IntCC::SignedGreaterThan, lhs, rhs),
            BinOp::GtEq => builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, lhs, rhs),
        }
    }
}

fn coerce_value(
    val: cranelift_codegen::ir::Value,
    target_ty: Type,
    builder: &mut FunctionBuilder,
) -> cranelift_codegen::ir::Value {
    let val_ty = builder.func.dfg.value_type(val);
    if val_ty == target_ty {
        return val;
    }
    if val_ty.is_int() && target_ty.is_int() {
        if val_ty.bits() > target_ty.bits() {
            return builder.ins().ireduce(target_ty, val);
        } else {
            return builder.ins().uextend(target_ty, val);
        }
    }
    if val_ty.is_float() && target_ty.is_float() {
        if val_ty.bits() > target_ty.bits() {
            return builder.ins().fdemote(target_ty, val);
        } else {
            return builder.ins().fpromote(target_ty, val);
        }
    }
    val
}

fn ty_to_cranelift(ty: &Ty) -> Option<Type> {
    match ty {
        Ty::Bool => Some(types::I8),
        Ty::Int8 | Ty::UInt8 => Some(types::I8),
        Ty::Int16 | Ty::UInt16 => Some(types::I16),
        Ty::Int32 | Ty::UInt32 | Ty::Char => Some(types::I32),
        Ty::Int | Ty::Int64 | Ty::UInt | Ty::UInt64 | Ty::ISize | Ty::USize => Some(types::I64),
        Ty::Float32 => Some(types::F32),
        Ty::Float | Ty::Float64 => Some(types::F64),
        Ty::String | Ty::Str | Ty::Vec(_) | Ty::HashMap(_, _) | Ty::Set(_)
        | Ty::Ref(_) | Ty::RefMut(_) | Ty::RefLifetime(_, _) | Ty::RefMutLifetime(_, _)
        | Ty::RawPtr(_) | Ty::RawPtrMut(_) | Ty::RawPtrVoid | Ty::RawPtrMutVoid
        | Ty::Option(_) | Ty::Result(_, _) | Ty::Class { .. } | Ty::Struct { .. }
        | Ty::Enum { .. } | Ty::Fn { .. } | Ty::FnMut { .. } | Ty::FnOnce { .. }
        | Ty::DynTrait(_) | Ty::ImplTrait(_) | Ty::Alias { .. } | Ty::Newtype { .. }
        | Ty::TypeParam { .. } | Ty::Infer(_) | Ty::Tuple(_) | Ty::Array(_, _) => Some(types::I64),
        Ty::Unit | Ty::Never => None,
        Ty::Error => None,
    }
}

fn cmpop_to_intcc(op: CmpOp) -> IntCC {
    match op {
        CmpOp::Eq => IntCC::Equal,
        CmpOp::NotEq => IntCC::NotEqual,
        CmpOp::Lt => IntCC::SignedLessThan,
        CmpOp::LtEq => IntCC::SignedLessThanOrEqual,
        CmpOp::Gt => IntCC::SignedGreaterThan,
        CmpOp::GtEq => IntCC::SignedGreaterThanOrEqual,
    }
}

fn is_string_typed_value(val: &MirValue, func: &MirFunction) -> bool {
    if let MirValue::Use(local_id) = val {
        if let Some(local) = func.locals.get(*local_id as usize) {
            return is_string_mir_ty(&local.ty);
        }
    }
    false
}

fn is_string_mir_ty(ty: &Ty) -> bool {
    match ty {
        Ty::String | Ty::Str => true,
        Ty::Ref(inner) | Ty::RefMut(inner)
        | Ty::RefLifetime(_, inner) | Ty::RefMutLifetime(_, inner) => is_string_mir_ty(inner),
        _ => false,
    }
}

fn simple_type_size(ty: &Ty) -> usize {
    match ty {
        Ty::Bool | Ty::Int8 | Ty::UInt8 => 1,
        Ty::Int16 | Ty::UInt16 => 2,
        Ty::Int32 | Ty::UInt32 | Ty::Float32 | Ty::Char => 4,
        Ty::Int | Ty::Int64 | Ty::UInt | Ty::UInt64 | Ty::ISize | Ty::USize
        | Ty::Float | Ty::Float64 => 8,
        Ty::String => 24,
        Ty::Str => 16,
        Ty::Vec(_) => 24,
        Ty::HashMap(_, _) | Ty::Set(_) => 48,
        Ty::Ref(_) | Ty::RefMut(_) | Ty::RefLifetime(_, _) | Ty::RefMutLifetime(_, _)
        | Ty::RawPtr(_) | Ty::RawPtrMut(_) | Ty::RawPtrVoid | Ty::RawPtrMutVoid => 8,
        Ty::Unit | Ty::Never => 0,
        Ty::Enum { .. } => 32,
        Ty::Class { .. } | Ty::Struct { .. } => 64,
        Ty::Option(_) => 16,
        Ty::Result(_, _) => 16,
        Ty::Tuple(elems) => elems.len().max(1) * 8,
        Ty::Array(_, n) => n * 8,
        _ => 8,
    }
}

fn runtime_signature(name: &str) -> Option<(Vec<Type>, Option<Type>)> {
    match name {
        "puts" | "riven_puts" => Some((vec![types::I64], None)),
        "eputs" | "riven_eputs" => Some((vec![types::I64], None)),
        "print" | "riven_print" => Some((vec![types::I64], None)),
        "riven_print_int" => Some((vec![types::I64], None)),
        "riven_int_to_string" => Some((vec![types::I64], Some(types::I64))),
        "riven_float_to_string" => Some((vec![types::F64], Some(types::I64))),
        "riven_bool_to_string" => Some((vec![types::I64], Some(types::I64))),
        "riven_string_concat" => Some((vec![types::I64, types::I64], Some(types::I64))),
        "riven_string_from" => Some((vec![types::I64], Some(types::I64))),
        "riven_string_push_str" => Some((vec![types::I64, types::I64], Some(types::I64))),
        "riven_string_len" => Some((vec![types::I64], Some(types::I64))),
        "riven_string_is_empty" => Some((vec![types::I64], Some(types::I8))),
        "riven_string_trim" => Some((vec![types::I64], Some(types::I64))),
        "riven_string_to_lower" => Some((vec![types::I64], Some(types::I64))),
        "riven_str_split" => Some((vec![types::I64, types::I64], Some(types::I64))),
        "riven_str_parse_uint" => Some((vec![types::I64], Some(types::I64))),
        "riven_alloc" => Some((vec![types::I64], Some(types::I64))),
        "riven_dealloc" => Some((vec![types::I64], None)),
        "riven_realloc" => Some((vec![types::I64, types::I64], Some(types::I64))),
        "riven_panic" => Some((vec![types::I64], None)),
        "riven_vec_new" => Some((vec![], Some(types::I64))),
        "riven_vec_push" => Some((vec![types::I64, types::I64], None)),
        "riven_vec_len" => Some((vec![types::I64], Some(types::I64))),
        "riven_vec_get" => Some((vec![types::I64, types::I64], Some(types::I64))),
        "riven_vec_get_opt" => Some((vec![types::I64, types::I64], Some(types::I64))),
        "riven_vec_get_mut" => Some((vec![types::I64, types::I64], Some(types::I64))),
        "riven_vec_get_mut_opt" => Some((vec![types::I64, types::I64], Some(types::I64))),
        "riven_vec_is_empty" => Some((vec![types::I64], Some(types::I8))),
        "riven_vec_each" => Some((vec![types::I64, types::I64], None)),
        "riven_option_unwrap_or" => Some((vec![types::I64, types::I64], Some(types::I64))),
        "riven_result_unwrap_or_else" => Some((vec![types::I64, types::I64], Some(types::I64))),
        "riven_result_try_op" => Some((vec![types::I64], Some(types::I64))),
        "riven_noop" => Some((vec![], None)),
        _ => None,
    }
}
