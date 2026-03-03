pub mod ast;
pub mod error;
pub mod eval;
pub mod functions;
pub mod lexer;
pub mod parser;
pub mod units;
pub mod value;

pub use eval::{evaluate_document, Environment, LineResult};
pub use value::Value;
pub use error::CalcError;

/// Convenience: evaluate a multi-line string and return one `LineResult` per line.
///
/// Errors in individual lines are reported per-line (the `error` field) rather
/// than aborting the whole evaluation. Lexer/parser errors that prevent parsing
/// the entire document are returned as a single-element vec with an error.
pub fn evaluate(input: &str) -> Vec<LineResult> {
    let mut lex = lexer::Lexer::new(input);
    let tokens = match lex.tokenize() {
        Ok(t) => t,
        Err(e) => {
            return vec![LineResult {
                value: None,
                display: String::new(),
                error: Some(e.to_string()),
                is_assignment: false,
            }];
        }
    };

    let mut parser = parser::Parser::new(tokens);
    let lines = match parser.parse_document() {
        Ok(l) => l,
        Err(e) => {
            return vec![LineResult {
                value: None,
                display: String::new(),
                error: Some(e.to_string()),
                is_assignment: false,
            }];
        }
    };

    evaluate_document(&lines)
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_simple_calc() {
        let results = evaluate("2 + 2");
        assert_eq!(results.len(), 1);
        assert!(results[0].error.is_none());
        assert_eq!(results[0].display, "4");
    }

    #[test]
    fn test_multiline_notebook() {
        let input = "salary = 5000\nrent = 1500\ngroceries = 400\ntotal_expenses = rent + groceries\nsavings = salary - total_expenses";
        let results = evaluate(input);
        let errors: Vec<_> = results.iter().filter(|r| r.error.is_some()).collect();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    #[test]
    fn test_unit_arithmetic() {
        let results = evaluate("5 km in m");
        assert!(results[0].error.is_none());
        let v = results[0].value.as_ref().unwrap();
        assert!((v.number - 5000.0).abs() < 1e-6);
    }

    #[test]
    fn test_percent_expressions() {
        let r = evaluate("20% of 150");
        assert_eq!(r[0].display, "30");

        let r = evaluate("200 + 15%");
        assert_eq!(r[0].display, "230");

        let r = evaluate("200 - 10%");
        assert_eq!(r[0].display, "180");
    }

    #[test]
    fn test_chained_assignments() {
        let r = evaluate("a = 10\nb = 20\na + b");
        assert!(r[2].error.is_none());
        assert_eq!(r[2].display, "30");
    }

    #[test]
    fn test_ans_reference() {
        let r = evaluate("100\nans * 2");
        assert_eq!(r[1].display, "200");
    }

    #[test]
    fn test_scientific_notation() {
        let r = evaluate("1e3 + 1e3");
        assert_eq!(r[0].display, "2000");
    }

    #[test]
    fn test_hex_literal() {
        let r = evaluate("0xFF");
        assert_eq!(r[0].display, "255");
    }

    #[test]
    fn test_binary_literal() {
        let r = evaluate("0b1111");
        assert_eq!(r[0].display, "15");
    }

    #[test]
    fn test_functions() {
        let r = evaluate("sqrt(144)");
        assert_eq!(r[0].display, "12");

        let r = evaluate("factorial(6)");
        assert_eq!(r[0].display, "720");

        let r = evaluate("min(3, 1, 4, 1, 5)");
        assert_eq!(r[0].display, "1");
    }

    #[test]
    fn test_nested_functions() {
        // sqrt(2) ≈ 1.41421356..., * 100 ≈ 141.421..., round = 141, /100 = 1.41
        let r = evaluate("round(sqrt(2) * 100) / 100");
        assert_eq!(r[0].display, "1.41");
    }

    #[test]
    fn test_error_recovery() {
        let r = evaluate("1 + 1\nundefined_var\n3 + 3");
        assert!(r[0].error.is_none());
        assert!(r[1].error.is_some());
        assert!(r[2].error.is_none());
        assert_eq!(r[2].display, "6");
    }

    #[test]
    fn test_temperature() {
        let r = evaluate("0 C in F");
        let v = r[0].value.as_ref().unwrap();
        assert!((v.number - 32.0).abs() < 1e-6);
    }

    #[test]
    fn test_data_units() {
        let r = evaluate("1 GB in MB");
        let v = r[0].value.as_ref().unwrap();
        assert!((v.number - 1024.0).abs() < 1e-6);
    }

    #[test]
    fn test_time_units() {
        let r = evaluate("2 h in min");
        let v = r[0].value.as_ref().unwrap();
        assert!((v.number - 120.0).abs() < 1e-6);
    }

    #[test]
    fn test_pi_constant() {
        let r = evaluate("pi");
        let v = r[0].value.as_ref().unwrap();
        assert!((v.number - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn test_right_assoc_power() {
        // 2^3^2 = 2^(3^2) = 2^9 = 512
        let r = evaluate("2^3^2");
        assert_eq!(r[0].display, "512");
    }

    #[test]
    fn test_comment_ignored() {
        let r = evaluate("# this is a comment\n42");
        assert!(r[0].error.is_none());
        assert!(r[0].value.is_none()); // comment has no value
        assert_eq!(r[1].display, "42");
    }

    #[test]
    fn test_label_ignored() {
        let r = evaluate("Budget:\n100 + 50");
        assert!(r[0].value.is_none());
        assert_eq!(r[1].display, "150");
    }
}
