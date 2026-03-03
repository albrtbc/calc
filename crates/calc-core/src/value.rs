use crate::units::Unit;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Value {
    pub number: f64,
    pub unit: Option<Unit>,
}

impl Value {
    pub fn new(number: f64) -> Self {
        Self { number, unit: None }
    }

    pub fn with_unit(number: f64, unit: Unit) -> Self {
        Self { number, unit: Some(unit) }
    }

    pub fn is_unitless(&self) -> bool {
        self.unit.is_none()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let num_str = format_number(self.number);
        if let Some(ref unit) = self.unit {
            write!(f, "{} {}", num_str, unit)
        } else {
            write!(f, "{}", num_str)
        }
    }
}

pub fn format_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() };
    }
    if n == n.trunc() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        // Up to 10 decimal places, trim trailing zeros
        let s = format!("{:.10}", n);
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number_integer() {
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(-7.0), "-7");
        assert_eq!(format_number(0.0), "0");
    }

    #[test]
    fn test_format_number_decimal() {
        assert_eq!(format_number(3.14), "3.14");
        assert_eq!(format_number(1.5), "1.5");
    }

    #[test]
    fn test_format_number_special() {
        assert_eq!(format_number(f64::NAN), "NaN");
        assert_eq!(format_number(f64::INFINITY), "Infinity");
        assert_eq!(format_number(f64::NEG_INFINITY), "-Infinity");
    }
}
