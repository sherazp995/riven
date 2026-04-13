//! Unit tests for HIR types and type context.

#[cfg(test)]
mod tests {
    use crate::hir::context::TypeContext;
    use crate::hir::types::{MoveSemantics, Ty};

    // ─── Copy/Move Classification ───────────────────────────────────

    #[test]
    fn int_is_copy() {
        assert!(Ty::Int.is_copy());
        assert_eq!(Ty::Int.move_semantics(), MoveSemantics::Copy);
    }

    #[test]
    fn all_primitive_integers_are_copy() {
        let ints = [
            Ty::Int, Ty::Int8, Ty::Int16, Ty::Int32, Ty::Int64,
            Ty::UInt, Ty::UInt8, Ty::UInt16, Ty::UInt32, Ty::UInt64,
            Ty::ISize, Ty::USize,
        ];
        for ty in &ints {
            assert!(ty.is_copy(), "{} should be Copy", ty);
        }
    }

    #[test]
    fn floats_are_copy() {
        assert!(Ty::Float.is_copy());
        assert!(Ty::Float32.is_copy());
        assert!(Ty::Float64.is_copy());
    }

    #[test]
    fn bool_and_char_are_copy() {
        assert!(Ty::Bool.is_copy());
        assert!(Ty::Char.is_copy());
    }

    #[test]
    fn unit_is_copy() {
        assert!(Ty::Unit.is_copy());
    }

    #[test]
    fn never_is_copy() {
        assert!(Ty::Never.is_copy());
    }

    #[test]
    fn immutable_ref_is_copy() {
        assert!(Ty::Ref(Box::new(Ty::String)).is_copy());
        assert!(Ty::Str.is_copy());
    }

    #[test]
    fn mutable_ref_is_move() {
        assert!(Ty::RefMut(Box::new(Ty::Int)).is_move());
    }

    #[test]
    fn string_is_move() {
        assert!(Ty::String.is_move());
        assert_eq!(Ty::String.move_semantics(), MoveSemantics::Move);
    }

    #[test]
    fn vec_is_move() {
        assert!(Ty::Vec(Box::new(Ty::Int)).is_move());
    }

    #[test]
    fn hash_is_move() {
        assert!(Ty::Hash(Box::new(Ty::String), Box::new(Ty::Int)).is_move());
    }

    #[test]
    fn tuple_of_copy_is_copy() {
        let tuple = Ty::Tuple(vec![Ty::Int, Ty::Bool, Ty::Char]);
        assert!(tuple.is_copy());
    }

    #[test]
    fn tuple_with_move_is_move() {
        let tuple = Ty::Tuple(vec![Ty::Int, Ty::String]);
        assert!(tuple.is_move());
    }

    #[test]
    fn array_of_copy_is_copy() {
        let array = Ty::Array(Box::new(Ty::Int), 10);
        assert!(array.is_copy());
    }

    #[test]
    fn array_of_move_is_move() {
        let array = Ty::Array(Box::new(Ty::String), 3);
        assert!(array.is_move());
    }

    #[test]
    fn class_is_move() {
        let cls = Ty::Class {
            name: "Task".to_string(),
            generic_args: vec![],
        };
        assert!(cls.is_move());
    }

    // ─── Type Queries ───────────────────────────────────────────────

    #[test]
    fn is_numeric() {
        assert!(Ty::Int.is_numeric());
        assert!(Ty::Float.is_numeric());
        assert!(Ty::USize.is_numeric());
        assert!(!Ty::Bool.is_numeric());
        assert!(!Ty::String.is_numeric());
    }

    #[test]
    fn is_integer() {
        assert!(Ty::Int.is_integer());
        assert!(Ty::UInt32.is_integer());
        assert!(!Ty::Float.is_integer());
    }

    #[test]
    fn is_float() {
        assert!(Ty::Float.is_float());
        assert!(Ty::Float32.is_float());
        assert!(!Ty::Int.is_float());
    }

    #[test]
    fn bit_width() {
        assert_eq!(Ty::Int8.bit_width(), Some(8));
        assert_eq!(Ty::Int16.bit_width(), Some(16));
        assert_eq!(Ty::Int32.bit_width(), Some(32));
        assert_eq!(Ty::Int64.bit_width(), Some(64));
        assert_eq!(Ty::Int.bit_width(), Some(64));
        assert_eq!(Ty::Bool.bit_width(), None);
    }

    #[test]
    fn deref_ty() {
        let inner = Ty::Int;
        let ref_ty = Ty::Ref(Box::new(inner.clone()));
        assert_eq!(ref_ty.deref_ty(), Some(&inner));
        assert_eq!(Ty::Int.deref_ty(), None);
    }

    #[test]
    fn is_option_and_result() {
        assert!(Ty::Option(Box::new(Ty::Int)).is_option());
        assert!(!Ty::Int.is_option());
        assert!(Ty::Result(Box::new(Ty::Int), Box::new(Ty::String)).is_result());
        assert!(!Ty::Int.is_result());
    }

    // ─── Type Display ───────────────────────────────────────────────

    #[test]
    fn display_primitives() {
        assert_eq!(format!("{}", Ty::Int), "Int");
        assert_eq!(format!("{}", Ty::Bool), "Bool");
        assert_eq!(format!("{}", Ty::Unit), "()");
        assert_eq!(format!("{}", Ty::Never), "Never");
        assert_eq!(format!("{}", Ty::String), "String");
        assert_eq!(format!("{}", Ty::Str), "&str");
    }

    #[test]
    fn display_composite() {
        assert_eq!(format!("{}", Ty::Vec(Box::new(Ty::Int))), "Vec[Int]");
        assert_eq!(
            format!("{}", Ty::Hash(Box::new(Ty::String), Box::new(Ty::Int))),
            "Hash[String, Int]"
        );
        assert_eq!(format!("{}", Ty::Option(Box::new(Ty::Int))), "Option[Int]");
        assert_eq!(format!("{}", Ty::Tuple(vec![Ty::Int, Ty::Bool])), "(Int, Bool)");
        assert_eq!(format!("{}", Ty::Ref(Box::new(Ty::Int))), "&Int");
        assert_eq!(format!("{}", Ty::RefMut(Box::new(Ty::Int))), "&mut Int");
    }

    #[test]
    fn display_class() {
        let cls = Ty::Class {
            name: "Task".to_string(),
            generic_args: vec![Ty::Int],
        };
        assert_eq!(format!("{}", cls), "Task[Int]");
    }

    // ─── TypeContext ────────────────────────────────────────────────

    #[test]
    fn fresh_type_var() {
        let mut ctx = TypeContext::new();
        let t0 = ctx.fresh_type_var();
        let t1 = ctx.fresh_type_var();
        assert_eq!(t0, Ty::Infer(0));
        assert_eq!(t1, Ty::Infer(1));
    }

    #[test]
    fn bind_and_resolve() {
        let mut ctx = TypeContext::new();
        let t0 = ctx.fresh_type_var();
        ctx.bind(0, Ty::Int).unwrap();
        let resolved = ctx.resolve(&t0);
        assert_eq!(resolved, Ty::Int);
    }

    #[test]
    fn transitive_resolution() {
        let mut ctx = TypeContext::new();
        let _t0 = ctx.fresh_type_var(); // ?T0
        let _t1 = ctx.fresh_type_var(); // ?T1
        ctx.bind(0, Ty::Infer(1)).unwrap(); // ?T0 = ?T1
        ctx.bind(1, Ty::Int).unwrap();       // ?T1 = Int
        let resolved = ctx.resolve(&Ty::Infer(0));
        assert_eq!(resolved, Ty::Int);
    }

    #[test]
    fn occurs_check() {
        let mut ctx = TypeContext::new();
        let _t0 = ctx.fresh_type_var();
        // Trying to bind ?T0 = Vec[?T0] should fail (infinite type)
        let result = ctx.bind(0, Ty::Vec(Box::new(Ty::Infer(0))));
        assert!(result.is_err());
    }

    #[test]
    fn is_fully_resolved() {
        let mut ctx = TypeContext::new();
        assert!(ctx.is_fully_resolved(&Ty::Int));
        assert!(!ctx.is_fully_resolved(&Ty::Infer(0)));

        let _t = ctx.fresh_type_var();
        ctx.bind(0, Ty::String).unwrap();
        assert!(ctx.is_fully_resolved(&Ty::Infer(0)));
    }

    #[test]
    fn resolve_composite() {
        let mut ctx = TypeContext::new();
        let _t0 = ctx.fresh_type_var();
        ctx.bind(0, Ty::Int).unwrap();
        let vec_ty = Ty::Vec(Box::new(Ty::Infer(0)));
        let resolved = ctx.resolve(&vec_ty);
        assert_eq!(resolved, Ty::Vec(Box::new(Ty::Int)));
    }
}
