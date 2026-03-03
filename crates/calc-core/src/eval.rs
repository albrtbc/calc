use std::collections::HashMap;

use crate::ast::{BinOp, Expr, Line};
use crate::error::{CalcError, Result};
use crate::functions::call_function;
use crate::units::Unit;
use crate::value::Value;

// ── Public result type ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LineResult {
    pub value: Option<Value>,
    pub display: String,
    pub error: Option<String>,
    pub is_assignment: bool,
}

impl LineResult {
    fn empty() -> Self {
        Self {
            value: None,
            display: String::new(),
            error: None,
            is_assignment: false,
        }
    }

    fn err(msg: String) -> Self {
        Self {
            value: None,
            display: String::new(),
            error: Some(msg),
            is_assignment: false,
        }
    }

    fn ok(value: Value, is_assignment: bool) -> Self {
        let display = value.to_string();
        Self {
            value: Some(value),
            display,
            error: None,
            is_assignment,
        }
    }
}

// ── Environment ───────────────────────────────────────────────────────────────

pub struct Environment {
    variables: HashMap<String, Value>,
    prev_result: Option<Value>,
    line_results: Vec<Option<Value>>,
}

impl Environment {
    pub fn new() -> Self {
        let mut env = Self {
            variables: HashMap::new(),
            prev_result: None,
            line_results: Vec::new(),
        };
        env.seed_constants();
        env
    }

    fn seed_constants(&mut self) {
        let consts: &[(&str, f64)] = &[
            ("pi", std::f64::consts::PI),
            ("PI", std::f64::consts::PI),
            ("e", std::f64::consts::E),
            ("E", std::f64::consts::E),
            ("tau", std::f64::consts::TAU),
            ("TAU", std::f64::consts::TAU),
            ("phi", 1.618_033_988_749_895),
            ("PHI", 1.618_033_988_749_895),
            ("inf", f64::INFINITY),
            ("infinity", f64::INFINITY),
            ("nan", f64::NAN),
        ];
        for (name, val) in consts {
            self.variables.insert(name.to_string(), Value::new(*val));
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    pub fn set(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    /// Record the result of line `line_index` (0-based) and update `ans`/`_`.
    fn record_result(&mut self, line_index: usize, value: Option<Value>) {
        // Ensure the vec is large enough
        while self.line_results.len() <= line_index {
            self.line_results.push(None);
        }
        self.line_results[line_index] = value.clone();

        if let Some(ref v) = value {
            // ans, _ = previous result
            self.prev_result = Some(v.clone());
            self.variables.insert("ans".to_string(), v.clone());
            self.variables.insert("_".to_string(), v.clone());
            // ans1, ans2, … and _1, _2, … (1-indexed)
            let key = format!("ans{}", line_index + 1);
            self.variables.insert(key, v.clone());
            let underscore_key = format!("_{}", line_index + 1);
            self.variables.insert(underscore_key, v.clone());
        }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn evaluate_document(lines: &[Line]) -> Vec<LineResult> {
    let mut env = Environment::new();
    let mut results = Vec::with_capacity(lines.len());

    for (i, line) in lines.iter().enumerate() {
        let result = evaluate_line(line, &mut env, i);
        let value = result.value.clone();
        env.record_result(i, value);
        results.push(result);
    }

    results
}

fn evaluate_line(line: &Line, env: &mut Environment, _line_index: usize) -> LineResult {
    match line {
        Line::Empty | Line::Label(_) => LineResult::empty(),
        Line::Comment(_) => LineResult::empty(),
        Line::Error(msg) => LineResult::err(msg.clone()),
        Line::Expr(expr) => {
            let is_assignment = matches!(expr, Expr::Assign { .. });
            match eval_expr(expr, env) {
                Ok(value) => LineResult::ok(value, is_assignment),
                Err(e) => LineResult::err(e.to_string()),
            }
        }
    }
}

// ── Expression evaluator ──────────────────────────────────────────────────────

fn eval_expr(expr: &Expr, env: &mut Environment) -> Result<Value> {
    match expr {
        Expr::Number(n) => Ok(Value::new(*n)),

        Expr::Ident(name) => {
            env.get(name)
                .cloned()
                .ok_or_else(|| CalcError::eval(format!("Undefined variable: '{}'", name)))
        }

        Expr::Assign { name, value } => {
            let v = eval_expr(value, env)?;
            env.set(name, v.clone());
            Ok(v)
        }

        Expr::UnaryNeg(inner) => {
            let mut v = eval_expr(inner, env)?;
            v.number = -v.number;
            Ok(v)
        }

        Expr::UnaryPos(inner) => eval_expr(inner, env),

        Expr::BinaryOp { op, left, right } => {
            eval_binary_op(*op, left, right, env)
        }

        Expr::FuncCall { name, args } => {
            let mut evaluated_args = Vec::with_capacity(args.len());
            for a in args {
                evaluated_args.push(eval_expr(a, env)?);
            }
            call_function(name, &evaluated_args)
        }

        Expr::Convert { expr, target_unit } => {
            let v = eval_expr(expr, env)?;
            convert_value(v, target_unit)
        }

        Expr::Percentage(inner) => {
            let v = eval_expr(inner, env)?;
            Ok(Value::new(v.number / 100.0))
        }

        Expr::PercentOf { percent, base } => {
            let pct = eval_expr(percent, env)?;
            let base_val = eval_expr(base, env)?;
            Ok(Value::new(pct.number / 100.0 * base_val.number))
        }

        Expr::PercentAdd { base, percent } => {
            let base_val = eval_expr(base, env)?;
            let pct = eval_expr(percent, env)?;
            let amount = base_val.number * pct.number / 100.0;
            let result = base_val.number + amount;
            Ok(Value { number: result, unit: base_val.unit })
        }

        Expr::PercentSub { base, percent } => {
            let base_val = eval_expr(base, env)?;
            let pct = eval_expr(percent, env)?;
            let amount = base_val.number * pct.number / 100.0;
            let result = base_val.number - amount;
            Ok(Value { number: result, unit: base_val.unit })
        }
    }
}

fn eval_binary_op(
    op: BinOp,
    left: &Expr,
    right: &Expr,
    env: &mut Environment,
) -> Result<Value> {
    let lv = eval_expr(left, env)?;
    let rv = eval_expr(right, env)?;

    // Unit handling for arithmetic:
    // - Add/Sub: units must match (or one must be unitless, in which case we
    //   preserve the other's unit).
    // - Mul/Div: result unit logic is simplified (we strip units for now, since
    //   compound units like m*s aren't in scope).
    // - Mod/Pow: unitless result.

    match op {
        BinOp::Add | BinOp::Sub => {
            // If both have the same unit → preserve unit
            // If one is unitless, preserve the other's unit
            // If both have different units → try to convert right to left's unit
            let (l, r, result_unit) = match (&lv.unit, &rv.unit) {
                (None, None) => (lv.number, rv.number, None),
                (Some(u), None) => (lv.number, rv.number, Some(u.clone())),
                (None, Some(u)) => (lv.number, rv.number, Some(u.clone())),
                (Some(lu), Some(ru)) => {
                    if lu == ru {
                        (lv.number, rv.number, Some(lu.clone()))
                    } else {
                        // Try to convert rv to lv's unit
                        let converted = Unit::convert(rv.number, ru, lu)?;
                        (lv.number, converted, Some(lu.clone()))
                    }
                }
            };
            let result = match op {
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
                _ => unreachable!(),
            };
            Ok(Value { number: result, unit: result_unit })
        }

        BinOp::Mul => {
            // If one operand is unitless, preserve the other's unit (scaling)
            // Otherwise strip units (compound units not supported)
            let result = lv.number * rv.number;
            let unit = match (&lv.unit, &rv.unit) {
                (None, None) => None,
                (Some(u), None) => Some(u.clone()),
                (None, Some(u)) => Some(u.clone()),
                (Some(_), Some(_)) => None, // compound unit — drop
            };
            Ok(Value { number: result, unit })
        }

        BinOp::Div => {
            if rv.number == 0.0 {
                return Err(CalcError::eval("Division by zero"));
            }
            let result = lv.number / rv.number;
            let unit = match (&lv.unit, &rv.unit) {
                (None, None) => None,
                (Some(u), None) => Some(u.clone()),
                (None, Some(_)) => None,
                (Some(_), Some(_)) => None, // ratio — drop
            };
            Ok(Value { number: result, unit })
        }

        BinOp::Mod => {
            if rv.number == 0.0 {
                return Err(CalcError::eval("Modulo by zero"));
            }
            Ok(Value::new(lv.number % rv.number))
        }

        BinOp::Pow => {
            Ok(Value::new(lv.number.powf(rv.number)))
        }
    }
}

/// Convert a value to a target unit. If the value has no unit, interpret it as
/// already being in the target unit's category base unit, or just attach the unit.
fn convert_value(v: Value, target_unit: &Unit) -> Result<Value> {
    match &v.unit {
        Some(src_unit) => {
            let converted = Unit::convert(v.number, src_unit, target_unit)?;
            Ok(Value::with_unit(converted, target_unit.clone()))
        }
        None => {
            // Unitless value: just attach the target unit (treat as literal "N unit")
            Ok(Value::with_unit(v.number, target_unit.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn eval(input: &str) -> Vec<LineResult> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let lines = parser.parse_document().unwrap();
        evaluate_document(&lines)
    }

    fn result_num(input: &str) -> f64 {
        let results = eval(input);
        let r = &results[0];
        if let Some(ref e) = r.error {
            panic!("Error evaluating '{}': {}", input, e);
        }
        r.value.as_ref().expect("Expected a value").number
    }

    #[test]
    fn test_basic_arithmetic() {
        assert!((result_num("1 + 2") - 3.0).abs() < 1e-10);
        assert!((result_num("10 - 3") - 7.0).abs() < 1e-10);
        assert!((result_num("4 * 5") - 20.0).abs() < 1e-10);
        assert!((result_num("15 / 3") - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_power() {
        assert!((result_num("2 ^ 10") - 1024.0).abs() < 1e-10);
        assert!((result_num("2 ^ 3 ^ 2") - 512.0).abs() < 1e-10); // right-assoc
    }

    #[test]
    fn test_constants() {
        let pi = result_num("pi");
        assert!((pi - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn test_assignment_and_variable() {
        let results = eval("x = 42\nx * 2");
        assert!(results[0].is_assignment);
        assert!((results[0].value.as_ref().unwrap().number - 42.0).abs() < 1e-10);
        assert!((results[1].value.as_ref().unwrap().number - 84.0).abs() < 1e-10);
    }

    #[test]
    fn test_ans() {
        let results = eval("5 + 5\nans + 1");
        assert!((results[1].value.as_ref().unwrap().number - 11.0).abs() < 1e-10);
    }

    #[test]
    fn test_percent_of() {
        assert!((result_num("20% of 500") - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_percent_add() {
        assert!((result_num("100 + 20%") - 120.0).abs() < 1e-10);
    }

    #[test]
    fn test_percent_sub() {
        assert!((result_num("100 - 20%") - 80.0).abs() < 1e-10);
    }

    #[test]
    fn test_unit_conversion() {
        let results = eval("1 km in m");
        let v = results[0].value.as_ref().unwrap();
        assert!((v.number - 1000.0).abs() < 1e-6);
        assert_eq!(v.unit, Some(Unit::Meter));
    }

    #[test]
    fn test_unit_literal() {
        let results = eval("5 km");
        let v = results[0].value.as_ref().unwrap();
        assert!((v.number - 5.0).abs() < 1e-10);
        assert_eq!(v.unit, Some(Unit::Kilometer));
    }

    #[test]
    fn test_division_by_zero() {
        let results = eval("1 / 0");
        assert!(results[0].error.is_some());
    }

    #[test]
    fn test_undefined_variable() {
        let results = eval("undefined_xyz");
        assert!(results[0].error.is_some());
    }

    #[test]
    fn test_multiline_independent() {
        let results = eval("1 + 1\nerror_line_xyz\n3 + 3");
        // First and third lines succeed, second fails
        assert!(results[0].error.is_none());
        assert!(results[1].error.is_some());
        assert!(results[2].error.is_none());
    }

    #[test]
    fn test_temperature_conversion() {
        let results = eval("100 C in F");
        let v = results[0].value.as_ref().unwrap();
        assert!((v.number - 212.0).abs() < 1e-6);
    }

    #[test]
    fn test_function_call() {
        assert!((result_num("sqrt(16)") - 4.0).abs() < 1e-10);
        assert!((result_num("abs(-5)") - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_ans_by_line_index() {
        let results = eval("10\n20\nans1 + ans2");
        let v = results[2].value.as_ref().unwrap();
        assert!((v.number - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_underscore_line_alias() {
        let results = eval("10\n20\n_1 + _2");
        let v = results[2].value.as_ref().unwrap();
        assert!((v.number - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_format_display() {
        let results = eval("100 + 20%");
        assert_eq!(results[0].display, "120");
    }
}
