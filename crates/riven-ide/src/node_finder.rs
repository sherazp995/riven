use riven_core::hir::nodes::*;
use riven_core::hir::types::Ty;
use riven_core::lexer::token::Span;

/// Describes the innermost HIR node found at a cursor position.
pub enum NodeAtPosition {
    VarRef(DefId, Span),
    FnCall { callee: DefId, span: Span },
    MethodCall { method: DefId, span: Span },
    FieldAccess { object_ty: Ty, field_name: String, span: Span },
    TypeRef { name: String, span: Span },
    Definition(DefId, Span),
}

/// Find the innermost HIR node whose span contains the given byte offset.
pub fn node_at_position(program: &HirProgram, byte_offset: usize) -> Option<NodeAtPosition> {
    let mut finder = NodeFinder {
        target: byte_offset,
        result: None,
    };
    finder.visit_program(program);
    finder.result
}

struct NodeFinder {
    target: usize,
    result: Option<NodeAtPosition>,
}

impl NodeFinder {
    fn contains(&self, span: &Span) -> bool {
        span.start <= self.target && self.target < span.end
    }

    fn visit_program(&mut self, program: &HirProgram) {
        for item in &program.items {
            self.visit_item(item);
        }
    }

    fn visit_item(&mut self, item: &HirItem) {
        match item {
            HirItem::Function(func) => self.visit_func_def(func),
            HirItem::Class(class) => {
                if self.contains(&class.span) {
                    // Check if cursor is on the class name
                    if let Some(name_span) = self.name_span_in_def(class.def_id, &class.name, &class.span) {
                        if self.contains(&name_span) {
                            self.result = Some(NodeAtPosition::Definition(class.def_id, name_span));
                        }
                    }
                    for field in &class.fields {
                        self.visit_field_def(field);
                    }
                    for method in &class.methods {
                        self.visit_func_def(method);
                    }
                    for imp in &class.impl_blocks {
                        self.visit_impl_block(imp);
                    }
                }
            }
            HirItem::Struct(s) => {
                if self.contains(&s.span) {
                    for field in &s.fields {
                        self.visit_field_def(field);
                    }
                }
            }
            HirItem::Enum(e) => {
                if self.contains(&e.span) {
                    for variant in &e.variants {
                        if self.contains(&variant.span) {
                            self.result =
                                Some(NodeAtPosition::Definition(variant.def_id, variant.span.clone()));
                        }
                    }
                }
            }
            HirItem::Trait(t) => {
                if self.contains(&t.span) {
                    for item in &t.items {
                        match item {
                            HirTraitItem::DefaultMethod(func) => self.visit_func_def(func),
                            HirTraitItem::MethodSig { span, .. } => {
                                if self.contains(span) {
                                    // method signature in trait
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            HirItem::Impl(imp) => self.visit_impl_block(imp),
            HirItem::Module(m) => {
                if self.contains(&m.span) {
                    for item in &m.items {
                        self.visit_item(item);
                    }
                }
            }
            HirItem::TypeAlias(_) | HirItem::Newtype(_) | HirItem::Const(_) => {}
        }
    }

    fn visit_impl_block(&mut self, imp: &HirImplBlock) {
        if self.contains(&imp.span) {
            for item in &imp.items {
                match item {
                    HirImplItem::Method(func) => self.visit_func_def(func),
                    HirImplItem::AssocType { .. } => {}
                }
            }
        }
    }

    fn visit_field_def(&mut self, field: &HirFieldDef) {
        if self.contains(&field.span) {
            self.result = Some(NodeAtPosition::Definition(field.def_id, field.span.clone()));
        }
    }

    fn visit_func_def(&mut self, func: &HirFuncDef) {
        if !self.contains(&func.span) {
            return;
        }
        // Check if cursor is on the function name itself
        // The name is at the start of the function definition
        self.result = Some(NodeAtPosition::Definition(func.def_id, func.span.clone()));

        for param in &func.params {
            if self.contains(&param.span) {
                self.result = Some(NodeAtPosition::Definition(param.def_id, param.span.clone()));
            }
        }
        self.visit_expr(&func.body);
    }

    fn visit_expr(&mut self, expr: &HirExpr) {
        if !self.contains(&expr.span) {
            return;
        }

        match &expr.kind {
            HirExprKind::VarRef(def_id) => {
                self.result = Some(NodeAtPosition::VarRef(*def_id, expr.span.clone()));
            }
            HirExprKind::FnCall {
                callee,
                args,
                ..
            } => {
                self.result = Some(NodeAtPosition::FnCall {
                    callee: *callee,
                    span: expr.span.clone(),
                });
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            HirExprKind::MethodCall {
                object,
                method,
                args,
                block,
                ..
            } => {
                self.visit_expr(object);
                // If we didn't find anything more specific in the object, this is a method call
                if !self.found_inside(object) {
                    self.result = Some(NodeAtPosition::MethodCall {
                        method: *method,
                        span: expr.span.clone(),
                    });
                }
                for arg in args {
                    self.visit_expr(arg);
                }
                if let Some(b) = block {
                    self.visit_expr(b);
                }
            }
            HirExprKind::FieldAccess {
                object,
                field_name,
                ..
            } => {
                self.visit_expr(object);
                if !self.found_inside(object) {
                    self.result = Some(NodeAtPosition::FieldAccess {
                        object_ty: object.ty.clone(),
                        field_name: field_name.clone(),
                        span: expr.span.clone(),
                    });
                }
            }
            HirExprKind::BinaryOp { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            HirExprKind::UnaryOp { operand, .. } => {
                self.visit_expr(operand);
            }
            HirExprKind::Borrow { expr: inner, .. } => {
                self.visit_expr(inner);
            }
            HirExprKind::Block(stmts, tail) => {
                for stmt in stmts {
                    self.visit_statement(stmt);
                }
                if let Some(tail) = tail {
                    self.visit_expr(tail);
                }
            }
            HirExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                self.visit_expr(cond);
                self.visit_expr(then_branch);
                if let Some(e) = else_branch {
                    self.visit_expr(e);
                }
            }
            HirExprKind::Match { scrutinee, arms } => {
                self.visit_expr(scrutinee);
                for arm in arms {
                    self.visit_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.visit_expr(guard);
                    }
                    self.visit_expr(&arm.body);
                }
            }
            HirExprKind::While { condition, body } => {
                self.visit_expr(condition);
                self.visit_expr(body);
            }
            HirExprKind::For {
                binding,
                iterable,
                body,
                ..
            } => {
                self.visit_expr(iterable);
                // The binding is a definition
                // We don't have a separate span for the binding name,
                // but it's part of the for expression
                let _ = binding;
                self.visit_expr(body);
            }
            HirExprKind::Loop { body } => {
                self.visit_expr(body);
            }
            HirExprKind::Assign { target, value, .. } => {
                self.visit_expr(target);
                self.visit_expr(value);
            }
            HirExprKind::CompoundAssign { target, value, .. } => {
                self.visit_expr(target);
                self.visit_expr(value);
            }
            HirExprKind::Return(Some(inner)) | HirExprKind::Break(Some(inner)) => {
                self.visit_expr(inner);
            }
            HirExprKind::Closure { body, .. } => {
                self.visit_expr(body);
            }
            HirExprKind::Construct { fields, .. } => {
                for (_name, val) in fields {
                    self.visit_expr(val);
                }
            }
            HirExprKind::EnumVariant { fields, .. } => {
                for (_name, val) in fields {
                    self.visit_expr(val);
                }
            }
            HirExprKind::Tuple(elems) | HirExprKind::ArrayLiteral(elems) => {
                for e in elems {
                    self.visit_expr(e);
                }
            }
            HirExprKind::Index { object, index } => {
                self.visit_expr(object);
                self.visit_expr(index);
            }
            HirExprKind::Cast { expr: inner, .. } => {
                self.visit_expr(inner);
            }
            HirExprKind::ArrayFill { value, .. } => {
                self.visit_expr(value);
            }
            HirExprKind::Range { start, end, .. } => {
                if let Some(s) = start {
                    self.visit_expr(s);
                }
                if let Some(e) = end {
                    self.visit_expr(e);
                }
            }
            HirExprKind::Interpolation { parts } => {
                for part in parts {
                    if let HirInterpolationPart::Expr(e) = part {
                        self.visit_expr(e);
                    }
                }
            }
            HirExprKind::MacroCall { args, .. } => {
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            HirExprKind::UnsafeBlock(stmts, tail) => {
                for stmt in stmts {
                    self.visit_statement(stmt);
                }
                if let Some(tail) = tail {
                    self.visit_expr(tail);
                }
            }
            // Literals and other terminal nodes — no children to visit
            HirExprKind::IntLiteral(_)
            | HirExprKind::FloatLiteral(_)
            | HirExprKind::StringLiteral(_)
            | HirExprKind::BoolLiteral(_)
            | HirExprKind::CharLiteral(_)
            | HirExprKind::UnitLiteral
            | HirExprKind::NullLiteral
            | HirExprKind::Continue
            | HirExprKind::Return(None)
            | HirExprKind::Break(None)
            | HirExprKind::Error => {}
        }
    }

    fn visit_statement(&mut self, stmt: &HirStatement) {
        match stmt {
            HirStatement::Let {
                def_id,
                value,
                span,
                pattern,
                ..
            } => {
                // Check if cursor is on the binding pattern
                if let Some(pat_span) = self.pattern_span(pattern) {
                    if self.contains(&pat_span) {
                        self.result = Some(NodeAtPosition::Definition(*def_id, pat_span));
                    }
                }
                if let Some(val) = value {
                    self.visit_expr(val);
                }
                // Fallback: if cursor is in the let statement span but not on value
                if self.contains(span) && self.result.is_none() {
                    self.result = Some(NodeAtPosition::Definition(*def_id, span.clone()));
                }
            }
            HirStatement::Expr(expr) => {
                self.visit_expr(expr);
            }
        }
    }

    fn visit_pattern(&mut self, pattern: &HirPattern) {
        match pattern {
            HirPattern::Binding {
                def_id, span, ..
            } => {
                if self.contains(span) {
                    self.result = Some(NodeAtPosition::Definition(*def_id, span.clone()));
                }
            }
            HirPattern::Tuple { elements, .. } => {
                for elem in elements {
                    self.visit_pattern(elem);
                }
            }
            HirPattern::Enum { fields, .. } => {
                for field in fields {
                    self.visit_pattern(field);
                }
            }
            HirPattern::Struct { fields, .. } => {
                for (_name, pat) in fields {
                    self.visit_pattern(pat);
                }
            }
            HirPattern::Or { patterns, .. } => {
                for pat in patterns {
                    self.visit_pattern(pat);
                }
            }
            HirPattern::Ref { def_id, span, .. } => {
                if self.contains(span) {
                    self.result = Some(NodeAtPosition::Definition(*def_id, span.clone()));
                }
            }
            HirPattern::Wildcard { .. }
            | HirPattern::Literal { .. }
            | HirPattern::Rest { .. } => {}
        }
    }

    fn pattern_span(&self, pattern: &HirPattern) -> Option<Span> {
        match pattern {
            HirPattern::Binding { span, .. } => Some(span.clone()),
            HirPattern::Tuple { span, .. } => Some(span.clone()),
            HirPattern::Wildcard { span } => Some(span.clone()),
            _ => None,
        }
    }

    /// Check if we already found a result inside a sub-expression.
    fn found_inside(&self, expr: &HirExpr) -> bool {
        if let Some(ref result) = self.result {
            let result_span = match result {
                NodeAtPosition::VarRef(_, s) => s,
                NodeAtPosition::FnCall { span, .. } => span,
                NodeAtPosition::MethodCall { span, .. } => span,
                NodeAtPosition::FieldAccess { span, .. } => span,
                NodeAtPosition::TypeRef { span, .. } => span,
                NodeAtPosition::Definition(_, s) => s,
            };
            // If the result span is strictly inside the given expression span,
            // we found something more specific
            result_span.start >= expr.span.start && result_span.end <= expr.span.end
                && (result_span.start != expr.span.start || result_span.end != expr.span.end)
        } else {
            false
        }
    }

    /// Try to compute a name-only span for a definition.
    /// For now, returns None — the full definition span is used instead.
    fn name_span_in_def(&self, _def_id: DefId, _name: &str, _outer_span: &Span) -> Option<Span> {
        None
    }
}
