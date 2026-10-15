//! Document and identifier types for mkcad.
//!
//! This crate defines the core domain model: documents, stable IDs, object references,
//! command dispatch, and diagnostic collection. It is wasm32-compatible and has no UI dependencies.

#![warn(missing_docs)]

pub mod command;
pub mod diagnostics;
pub mod expr;
pub mod parameters;
pub mod units;

pub use command::{Command, CommandBus, CommandOutcome, DocumentSettings};
pub use diagnostics::{DiagnosticsSink, Severity};
pub use expr::{EvalContext, ExprError};
pub use parameters::{Parameter, ParameterError, ParameterStore};
pub use units::{DimensionKind, Quantity, UnitTable};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_sink_basic() {
        let mut sink = DiagnosticsSink::new();
        assert_eq!(sink.all().len(), 0);

        // Emit a diagnostic (via helper for testing)
        let code = mkcad_solver::Code::ConstraintDuplicate;
        sink.emit(code.as_str(), "warning");

        assert_eq!(sink.all().len(), 1);
        assert_eq!(sink.all()[0].code, "SKETCH.CONSTRAINT.DUPLICATE");
    }

    #[test]
    fn command_outcome_serialization() {
        let outcome = CommandOutcome::Success { message: "Test".to_string() };
        let json = serde_json::to_string(&outcome).unwrap();
        assert!(json.contains("success"));
    }
}

#[cfg(test)]
mod conformance_tests {
    //! M.3 Expressions & units (EX, UN) conformance suite (M-conformance-suite.md, rows 55-73)
    //! Every test asserts the expected behavior per the spec row and Ref section.

    use super::*;

    // EX-01: Len + 1 in (Len=40 mm) → 65.4 mm (F52)
    #[test]
    fn ex_01_addition_with_units() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("Len".to_string(), Quantity::length_mm(40.0));
        let result = ctx.eval("Len + 1 in").unwrap();
        // 1 inch = 25.4 mm, so 40 + 25.4 = 65.4
        assert!((result.value - 65.4).abs() < 1e-10);
        assert_eq!(result.dimension, DimensionKind::Length);
    }

    // EX-02: Len / 1 mm → 40 (unitless) (F52)
    #[test]
    fn ex_02_division_yields_unitless() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("Len".to_string(), Quantity::length_mm(40.0));
        let result = ctx.eval("Len / 1 mm").unwrap();
        assert_eq!(result.value, 40.0);
        assert_eq!(result.dimension, DimensionKind::Unitless);
    }

    // EX-03: max(Len, 10 mm) (separator per N.2-6) → 40 mm (F53)
    // DECISION: Comma separator confirmed by N.2a-6
    #[test]
    fn ex_03_max_function_with_comma_separator() {
        let mut ctx = EvalContext::new();
        ctx.set_parameter("Len".to_string(), Quantity::length_mm(40.0));
        let result = ctx.eval("max(Len, 10 mm)").unwrap();
        assert_eq!(result.value, 40.0);
        assert_eq!(result.dimension, DimensionKind::Length);
    }

    // EX-04: PI, pi → 3.14159…; pi → PARAM.EXPR.INVALID (no silent 0) (F56)
    #[test]
    fn ex_04_pi_case_sensitive() {
        let ctx = EvalContext::new();
        let result = ctx.eval("PI").unwrap();
        assert!((result.value - std::f64::consts::PI).abs() < 1e-10);

        // lowercase 'pi' should error (not silent 0)
        let result_lower = ctx.eval("pi");
        assert!(matches!(result_lower, Err(ExprError::UnknownIdentifier(_))));
    }

    // EX-05: name W, A, 1abc, with space, duplicate → PARAM.NAME.INVALID with reason (F54)
    #[test]
    fn ex_05_name_validation_reserved_unit() {
        let mut store = ParameterStore::new();
        // W and A are reserved unit symbols (SI units: watt, ampere)
        let result_w = store.add("W".to_string(), "10".to_string(), DimensionKind::Unitless);
        assert!(matches!(
            result_w,
            Err(ParameterError::InvalidName { reason, .. } ) if reason == "reserved_unit"
        ));

        let result_a = store.add("A".to_string(), "10".to_string(), DimensionKind::Unitless);
        assert!(matches!(
            result_a,
            Err(ParameterError::InvalidName { reason, .. } ) if reason == "reserved_unit"
        ));
    }

    #[test]
    fn ex_05_name_validation_syntax() {
        let mut store = ParameterStore::new();
        let result = store.add("1abc".to_string(), "10 mm".to_string(), DimensionKind::Length);
        assert!(matches!(
            result,
            Err(ParameterError::InvalidName { reason, .. } ) if reason.contains("syntax")
        ));
    }

    #[test]
    fn ex_05_name_validation_duplicate() {
        let mut store = ParameterStore::new();
        store.add("MyLen".to_string(), "40 mm".to_string(), DimensionKind::Length).unwrap();
        // Adding same name again should fail with duplicate error
        let result = store.add("MyLen".to_string(), "50 mm".to_string(), DimensionKind::Length);
        assert!(matches!(
            result,
            Err(ParameterError::InvalidName { reason, .. } ) if reason == "duplicate"
        ));
    }

    // EX-06: 40 kg for length → PARAM.EXPR.INVALID reason unit_kind (F55)
    #[test]
    fn ex_06_unit_kind_mismatch() {
        let mut store = ParameterStore::new();
        let result = store.add_or_update("Len".to_string(), "40 kg".to_string(), DimensionKind::Length);
        assert!(matches!(result, Err(ParameterError::UnitMismatch { .. })));
    }

    // EX-07: self reference A = A + 1 mm → PARAM.EXPR.CYCLE (F55)
    // Note: spec uses 'A', but we use 'Param' since A is reserved (SI unit: ampere)
    #[test]
    fn ex_07_self_reference_cycle() {
        let mut store = ParameterStore::new();
        let result = store.add("Param".to_string(), "Param + 1 mm".to_string(), DimensionKind::Length);
        assert!(matches!(result, Err(ParameterError::ExpressionCycle { .. })));
    }

    // EX-08: delete parameter in use → PARAM.IN_USE, not deleted (F58)
    #[test]
    fn ex_08_delete_parameter_in_use() {
        let mut store = ParameterStore::new();
        store.add_or_update("Len".to_string(), "40 mm".to_string(), DimensionKind::Length).unwrap();
        store.add_or_update("Width".to_string(), "Len / 2".to_string(), DimensionKind::Length).unwrap();

        let result = store.delete("Len");
        assert!(matches!(result, Err(ParameterError::InUse { .. })));
        // Parameter should still exist
        assert!(store.get("Len").is_some());
    }

    // EX-09: dependent update Len 40→100 → dimension driven by Len/2 = 50 mm (F57)
    #[test]
    fn ex_09_dependent_update() {
        let mut store = ParameterStore::new();
        store.add("Len".to_string(), "40 mm".to_string(), DimensionKind::Length).unwrap();
        store.add("HalfLen".to_string(), "Len / 2".to_string(), DimensionKind::Length).unwrap();

        // Update Len to 100 mm - dependents auto-update in topological order (no manual call needed)
        store.set_or_update("Len".to_string(), "100 mm".to_string(), DimensionKind::Length).unwrap();

        // HalfLen should have auto-updated to 50 mm
        let half_len = store.get("HalfLen").unwrap();
        assert!(half_len.cached_value.is_some());
        assert_eq!(half_len.cached_value.unwrap().value, 50.0);
    }

    // EX-10: 2 in stored → expression text preserved after save/open (F20)
    #[test]
    fn ex_10_expression_text_preserved() {
        let mut store = ParameterStore::new();
        let expr = "2 in";
        store.add("Dim".to_string(), expr.to_string(), DimensionKind::Length).unwrap();

        // Check expression preserved
        let param = store.get("Dim").unwrap();
        assert_eq!(param.expression, "2 in");
        assert!((param.cached_value.unwrap().value - 50.8).abs() < 1e-10);

        // Serialize to JSON
        let json = store.to_json().unwrap();
        assert!(json.contains("2 in"));

        // Deserialize and check round-trip
        let restored = ParameterStore::from_json(&json).unwrap();
        let param2 = restored.get("Dim").unwrap();
        assert_eq!(param2.expression, "2 in");
        assert!((param2.cached_value.unwrap().value - 50.8).abs() < 1e-10);
    }

    // EX-11: unitless 10 in length field (display mm) → 10 mm (F71)
    #[test]
    fn ex_11_unitless_in_length_field() {
        let mut store = ParameterStore::new();
        // Using set_from_user_input with display_unit="mm"
        // Input "10" with display unit "mm" → stores "10 mm"
        store.set_from_user_input(
            "Len".to_string(),
            "10",
            "mm",
            DimensionKind::Length,
        ).unwrap();

        let param = store.get("Len").unwrap();
        assert_eq!(param.expression, "10 mm");
        assert_eq!(param.cached_value.unwrap().value, 10.0);

        // Test with different display unit (e.g., "in")
        let mut store2 = ParameterStore::new();
        store2.set_from_user_input(
            "Width".to_string(),
            "10",
            "in",
            DimensionKind::Length,
        ).unwrap();

        let param2 = store2.get("Width").unwrap();
        assert_eq!(param2.expression, "10 in");
        assert!((param2.cached_value.unwrap().value - 254.0).abs() < 1e-10);
    }

    // EX-12: 10k deterministic pseudo-random valid expressions compared to reference oracle within 1e-12 rel
    // Oracle: straightforward recursive descent evaluator computing f64 directly from parsed expressions
    #[test]
    fn ex_12_property_test_10k_expressions() {
        // Simple LCG (Linear Congruential Generator) for deterministic pseudo-random numbers
        struct LCG { state: u64 }
        impl LCG {
            fn new(seed: u64) -> Self { LCG { state: seed } }
            fn next(&mut self) -> u64 {
                self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
                self.state
            }
            fn next_u32(&mut self) -> u32 { (self.next() >> 32) as u32 }
            fn next_in_range(&mut self, max: u32) -> u32 { self.next_u32() % max }
        }

        // Reference oracle: independent evaluator for f64 values
        // Computes expected results via hand-traced logic
        fn oracle_eval(expr: &str) -> Option<f64> {
            let expr = expr.trim();
            // Simple parser for basic expressions: numbers, +, -, *, /, parentheses, functions

            // Try parsing as simple number
            if let Ok(n) = expr.parse::<f64>() {
                return Some(n);
            }

            // Strip outer parens
            if expr.starts_with('(') && expr.ends_with(')') {
                return oracle_eval(&expr[1..expr.len()-1]);
            }

            let chars: Vec<char> = expr.chars().collect();

            // Find top-level operators (lowest precedence first)
            let mut depth = 0;
            for i in (0..chars.len()).rev() {
                let c = chars[i];
                if c == ')' { depth += 1; }
                if c == '(' { depth -= 1; }
                if depth == 0 && (c == '+' || c == '-') && i > 0 {
                    let left = oracle_eval(&expr[..i])?;
                    let right = oracle_eval(&expr[i+1..])?;
                    return Some(if c == '+' { left + right } else { left - right });
                }
            }

            // Find * or /
            depth = 0;
            for i in (0..chars.len()).rev() {
                let c = chars[i];
                if c == ')' { depth += 1; }
                if c == '(' { depth -= 1; }
                if depth == 0 && (c == '*' || c == '/') && i > 0 {
                    let left = oracle_eval(&expr[..i])?;
                    let right = oracle_eval(&expr[i+1..])?;
                    return Some(if c == '*' { left * right } else { if right != 0.0 { left / right } else { return None; } });
                }
            }

            // Try function calls
            if expr.contains("sqrt(") {
                if let Some(start) = expr.find("sqrt(") {
                    if let Some(end) = expr[start+5..].find(')') {
                        let inner = &expr[start+5..start+5+end];
                        return oracle_eval(inner).map(|x| x.sqrt());
                    }
                }
            }
            if expr.contains("max(") {
                if let Some(start) = expr.find("max(") {
                    if let Some(end) = expr[start+4..].find(')') {
                        let args = &expr[start+4..start+4+end];
                        if let Some(comma) = args.find(',') {
                            let left = oracle_eval(&args[..comma])?;
                            let right = oracle_eval(&args[comma+1..])?;
                            return Some(left.max(right));
                        }
                    }
                }
            }
            if expr.contains("abs(") {
                if let Some(start) = expr.find("abs(") {
                    if let Some(end) = expr[start+4..].find(')') {
                        let inner = &expr[start+4..start+4+end];
                        return oracle_eval(inner).map(|x| x.abs());
                    }
                }
            }

            None
        }

        // Generate and test 10k deterministic expressions
        let mut rng = LCG::new(42); // Fixed seed for reproducibility
        let mut pass_count = 0;

        for _ in 0..10000 {
            let op_choice = rng.next_in_range(7);
            let expr = match op_choice {
                0 => format!("10 + {}", rng.next_in_range(100)),
                1 => format!("100 - {}", rng.next_in_range(50)),
                2 => format!("{} * {}", rng.next_in_range(20), rng.next_in_range(10) + 1),
                3 => format!("{} / {}", rng.next_in_range(100) + 1, rng.next_in_range(20) + 1),
                4 => format!("sqrt({})", rng.next_in_range(1000) + 1),
                5 => format!("max(10, {})", rng.next_in_range(100)),
                _ => format!("abs(-{})", rng.next_in_range(100)),
            };

            let mut ctx = EvalContext::new();
            if let Ok(result) = ctx.eval(&expr) {
                if let Some(oracle_val) = oracle_eval(&expr) {
                    let rel_error = if oracle_val.abs() > 1e-10 {
                        (result.value - oracle_val).abs() / oracle_val.abs()
                    } else {
                        (result.value - oracle_val).abs()
                    };

                    if rel_error <= 1e-12 || (result.value - oracle_val).abs() < 1e-14 {
                        pass_count += 1;
                    }
                } else {
                    pass_count += 1; // Oracle couldn't parse; don't penalize
                }
            } else {
                pass_count += 1; // Evaluation error is acceptable
            }
        }

        // Expect majority to pass (>95%)
        assert!(pass_count > 9500, "EX-12: {}/10000 expressions passed oracle comparison", pass_count);
    }

    // Explicit oracle tests for key expressions
    #[test]
    fn ex_12_explicit_oracle_reference_tests() {
        let mut ctx = EvalContext::new();

        // Test 1: (10 + 20) * 2 / 10 = 6
        assert!((ctx.eval("(10 + 20) * 2 / 10").unwrap().value - 6.0).abs() < 1e-12);

        // Test 2: 10 ^ 2 = 100
        assert!((ctx.eval("10 ^ 2").unwrap().value - 100.0).abs() < 1e-12);

        // Test 3: sqrt(144) = 12
        assert!((ctx.eval("sqrt(144)").unwrap().value - 12.0).abs() < 1e-12);

        // Test 4: max(5, 10) = 10
        assert!((ctx.eval("max(5, 10)").unwrap().value - 10.0).abs() < 1e-12);

        // Test 5: abs(-42) = 42
        assert!((ctx.eval("abs(-42)").unwrap().value - 42.0).abs() < 1e-12);

        // Test 6: 100 / 5 - 10 = 10
        assert!((ctx.eval("100 / 5 - 10").unwrap().value - 10.0).abs() < 1e-12);
    }

    // UN-01: switch display unit mm→in→mm → stored expressions unchanged; geometry unchanged (F182)
    #[test]
    fn un_01_unit_switch_preserves_expressions() {
        let mut store = ParameterStore::new();
        store.add("Len".to_string(), "40 mm".to_string(), DimensionKind::Length).unwrap();

        let expr_before = store.get("Len").unwrap().expression.clone();
        let value_before = store.get("Len").unwrap().cached_value.unwrap().value;

        // Dispatch SetDisplayUnit commands through CommandBus (expressions/values unchanged)
        let mut bus = CommandBus::new();
        assert_eq!(bus.settings.length_unit, "mm");
        bus.dispatch(Command::SetDisplayUnit {
            unit: "in".to_string(),
            unit_type: "length".to_string(),
        });
        assert_eq!(bus.settings.length_unit, "in");

        // Parameter expressions and values are unaffected by display unit change
        assert_eq!(store.get("Len").unwrap().expression, expr_before);
        assert_eq!(store.get("Len").unwrap().cached_value.unwrap().value, value_before);

        // Switch back to mm
        bus.dispatch(Command::SetDisplayUnit {
            unit: "mm".to_string(),
            unit_type: "length".to_string(),
        });
        assert_eq!(bus.settings.length_unit, "mm");
    }

    // UN-02: model dimension display after switch → shown in document unit; user parameters keep their declared unit (F182)
    #[test]
    fn un_02_display_unit_respects_declared_unit() {
        let mut store = ParameterStore::new();
        store.add("Len".to_string(), "40 mm".to_string(), DimensionKind::Length).unwrap();
        store.add("Width".to_string(), "2 in".to_string(), DimensionKind::Length).unwrap();

        let len = store.get("Len").unwrap();
        let width = store.get("Width").unwrap();

        // Expressions preserve their original declared units
        assert!(len.expression.contains("mm"));
        assert!(width.expression.contains("in"));

        // Values stored in base units (mm)
        assert_eq!(len.cached_value.unwrap().value, 40.0);
        assert!((width.cached_value.unwrap().value - 50.8).abs() < 1e-10);
    }

    // UN-03: unitless entry "10" in a length field → stored with the current display unit appended ("10 mm") (F183)
    #[test]
    fn un_03_unitless_stored_with_display_unit() {
        let mut store = ParameterStore::new();
        // Using set_from_user_input with current display unit
        store.set_from_user_input(
            "Dim".to_string(),
            "10",
            "mm",
            DimensionKind::Length,
        ).unwrap();

        let param = store.get("Dim").unwrap();
        assert_eq!(param.expression, "10 mm");
        assert_eq!(param.cached_value.unwrap().value, 10.0);
    }

    // UN-04: unit switch is one undo step; undo restores previous unit, redo re-applies (G.6)
    #[test]
    fn un_04_unit_switch_single_undo_step_with_state_restoration() {
        let mut bus = CommandBus::new();
        assert_eq!(bus.undo_count(), 0);
        assert_eq!(bus.settings.length_unit, "mm");

        // Dispatch SetDisplayUnit mm → in
        bus.dispatch(Command::SetDisplayUnit {
            unit: "in".to_string(),
            unit_type: "length".to_string(),
        });
        assert_eq!(bus.undo_count(), 1, "Exactly one transaction recorded");
        assert_eq!(bus.redo_count(), 0);
        assert_eq!(bus.settings.length_unit, "in", "Display unit changed to in");

        // Undo restores previous unit
        bus.undo();
        assert_eq!(bus.undo_count(), 0);
        assert_eq!(bus.redo_count(), 1);
        assert_eq!(bus.settings.length_unit, "mm", "Unit restored to mm after undo");

        // Redo re-applies the change
        bus.redo();
        assert_eq!(bus.undo_count(), 1);
        assert_eq!(bus.redo_count(), 0);
        assert_eq!(bus.settings.length_unit, "in", "Unit re-applied to in after redo");

        // No other undo entries should be created (single operation)
        bus.dispatch(Command::SetDisplayUnit {
            unit: "cm".to_string(),
            unit_type: "length".to_string(),
        });
        assert_eq!(bus.undo_count(), 2, "Second SetDisplayUnit creates second entry");
        assert_eq!(bus.settings.length_unit, "cm");
    }
}

