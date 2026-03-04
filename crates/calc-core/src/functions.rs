use crate::error::{CalcError, Result};
use crate::value::Value;

/// Dispatch a built-in function call.
/// All functions strip units from inputs and return unitless values
/// (unless the function is a unit-aware wrapper, which none currently are).
pub fn call_function(name: &str, args: &[Value]) -> Result<Value> {
    match name {
        // ── Trigonometric (single-argument, radians) ──────────────────────────
        "sin" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.sin()))
        }
        "cos" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.cos()))
        }
        "tan" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.tan()))
        }
        "asin" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.asin()))
        }
        "acos" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.acos()))
        }
        "atan" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.atan()))
        }
        "atan2" => {
            let (y, x) = two_args(name, args)?;
            Ok(Value::new(y.atan2(x)))
        }

        // ── Roots and powers ──────────────────────────────────────────────────
        "sqrt" => {
            let x = single_arg(name, args)?;
            if x < 0.0 {
                return Err(CalcError::eval("sqrt of negative number"));
            }
            Ok(Value::new(x.sqrt()))
        }
        "cbrt" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.cbrt()))
        }
        "pow" => {
            let (base, exp) = two_args(name, args)?;
            Ok(Value::new(base.powf(exp)))
        }

        // ── Logarithms ───────────────────────────────────────────────────────
        "log10" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.log10()))
        }
        "log2" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.log2()))
        }
        "ln" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.ln()))
        }
        "log" => {
            match args.len() {
                1 => Ok(Value::new(args[0].number.log10())),
                2 => {
                    let x = args[0].number;
                    let base = args[1].number;
                    if base <= 0.0 || base == 1.0 {
                        return Err(CalcError::eval(format!(
                            "log: invalid base {}",
                            base
                        )));
                    }
                    Ok(Value::new(x.log(base)))
                }
                n => Err(CalcError::eval(format!(
                    "log expects 1 or 2 arguments, got {}",
                    n
                ))),
            }
        }

        // ── Rounding ──────────────────────────────────────────────────────────
        "ceil" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.ceil()))
        }
        "floor" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.floor()))
        }
        "round" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.round()))
        }
        "trunc" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.trunc()))
        }

        // ── Miscellaneous ────────────────────────────────────────────────────
        "abs" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.abs()))
        }
        "exp" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.exp()))
        }
        "min" => {
            if args.is_empty() {
                return Err(CalcError::eval("min requires at least one argument"));
            }
            let result = args.iter().map(|a| a.number).fold(f64::INFINITY, f64::min);
            Ok(Value::new(result))
        }
        "max" => {
            if args.is_empty() {
                return Err(CalcError::eval("max requires at least one argument"));
            }
            let result = args.iter().map(|a| a.number).fold(f64::NEG_INFINITY, f64::max);
            Ok(Value::new(result))
        }
        "factorial" | "fact" => {
            let x = single_arg(name, args)?;
            if x < 0.0 || x != x.trunc() {
                return Err(CalcError::eval(format!(
                    "factorial requires a non-negative integer, got {}",
                    x
                )));
            }
            let n = x as u64;
            if n > 170 {
                return Err(CalcError::eval(format!(
                    "factorial({}) overflows f64 (max 170)",
                    n
                )));
            }
            Ok(Value::new(factorial(n)))
        }
        "sign" | "signum" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.signum()))
        }
        "clamp" => {
            if args.len() != 3 {
                return Err(CalcError::eval(format!(
                    "clamp requires 3 arguments (value, min, max), got {}",
                    args.len()
                )));
            }
            let v = args[0].number;
            let lo = args[1].number;
            let hi = args[2].number;
            Ok(Value::new(v.clamp(lo, hi)))
        }
        "hypot" => {
            let (x, y) = two_args(name, args)?;
            Ok(Value::new(x.hypot(y)))
        }
        "frac" => {
            let x = single_arg(name, args)?;
            Ok(Value::new(x.fract()))
        }
        "gcd" => {
            let (a, b) = two_args(name, args)?;
            if a != a.trunc() || b != b.trunc() {
                return Err(CalcError::eval("gcd requires integer arguments"));
            }
            Ok(Value::new(gcd(a.abs() as u64, b.abs() as u64) as f64))
        }
        "lcm" => {
            let (a, b) = two_args(name, args)?;
            if a != a.trunc() || b != b.trunc() {
                return Err(CalcError::eval("lcm requires integer arguments"));
            }
            let a = a.abs() as u64;
            let b = b.abs() as u64;
            let g = gcd(a, b);
            if g == 0 {
                Ok(Value::new(0.0))
            } else {
                Ok(Value::new((a / g * b) as f64))
            }
        }
        "mod" => {
            let (a, b) = two_args(name, args)?;
            if b == 0.0 {
                return Err(CalcError::eval("mod: division by zero"));
            }
            Ok(Value::new(a % b))
        }

        unknown => Err(CalcError::eval(format!("Unknown function: '{}'", unknown))),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn single_arg(name: &str, args: &[Value]) -> Result<f64> {
    if args.len() != 1 {
        return Err(CalcError::eval(format!(
            "{} expects 1 argument, got {}",
            name,
            args.len()
        )));
    }
    Ok(args[0].number)
}

fn two_args(name: &str, args: &[Value]) -> Result<(f64, f64)> {
    if args.len() != 2 {
        return Err(CalcError::eval(format!(
            "{} expects 2 arguments, got {}",
            name,
            args.len()
        )));
    }
    Ok((args[0].number, args[1].number))
}

fn factorial(n: u64) -> f64 {
    if n == 0 || n == 1 {
        return 1.0;
    }
    let mut acc = 1.0f64;
    for i in 2..=n {
        acc *= i as f64;
    }
    acc
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(n: f64) -> Value {
        Value::new(n)
    }

    #[test]
    fn test_sin() {
        let r = call_function("sin", &[v(0.0)]).unwrap();
        assert!((r.number - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_sqrt() {
        let r = call_function("sqrt", &[v(16.0)]).unwrap();
        assert!((r.number - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_sqrt_negative() {
        assert!(call_function("sqrt", &[v(-1.0)]).is_err());
    }

    #[test]
    fn test_factorial() {
        let r = call_function("factorial", &[v(5.0)]).unwrap();
        assert!((r.number - 120.0).abs() < 1e-10);
    }

    #[test]
    fn test_factorial_overflow() {
        assert!(call_function("factorial", &[v(171.0)]).is_err());
    }

    #[test]
    fn test_min_max() {
        let r = call_function("min", &[v(3.0), v(1.0), v(4.0)]).unwrap();
        assert!((r.number - 1.0).abs() < 1e-10);
        let r = call_function("max", &[v(3.0), v(1.0), v(4.0)]).unwrap();
        assert!((r.number - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_log() {
        let r = call_function("log", &[v(100.0)]).unwrap();
        assert!((r.number - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_log_base() {
        let r = call_function("log", &[v(8.0), v(2.0)]).unwrap();
        assert!((r.number - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_unknown() {
        assert!(call_function("unknown_func", &[v(1.0)]).is_err());
    }

    #[test]
    fn test_gcd() {
        let r = call_function("gcd", &[v(12.0), v(8.0)]).unwrap();
        assert!((r.number - 4.0).abs() < 1e-10);
    }
}
