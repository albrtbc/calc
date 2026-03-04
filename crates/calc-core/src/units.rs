use std::fmt;
use crate::error::{CalcError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    // Length
    Meter,
    Kilometer,
    Centimeter,
    Millimeter,
    Inch,
    Foot,
    Yard,
    Mile,
    // Weight / Mass
    Kilogram,
    Gram,
    Milligram,
    Pound,
    Ounce,
    // Temperature
    Celsius,
    Fahrenheit,
    Kelvin,
    // Data
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    // Time
    Second,
    Minute,
    Hour,
    Day,
    // Variable-based custom unit (name of a variable used as conversion rate)
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnitCategory {
    Length,
    Mass,
    Temperature,
    Data,
    Time,
    Custom,
}

impl Unit {
    pub fn category(&self) -> UnitCategory {
        match self {
            Unit::Meter
            | Unit::Kilometer
            | Unit::Centimeter
            | Unit::Millimeter
            | Unit::Inch
            | Unit::Foot
            | Unit::Yard
            | Unit::Mile => UnitCategory::Length,

            Unit::Kilogram
            | Unit::Gram
            | Unit::Milligram
            | Unit::Pound
            | Unit::Ounce => UnitCategory::Mass,

            Unit::Celsius | Unit::Fahrenheit | Unit::Kelvin => UnitCategory::Temperature,

            Unit::Byte
            | Unit::Kilobyte
            | Unit::Megabyte
            | Unit::Gigabyte
            | Unit::Terabyte => UnitCategory::Data,

            Unit::Second | Unit::Minute | Unit::Hour | Unit::Day => UnitCategory::Time,
            Unit::Custom(_) => UnitCategory::Custom,
        }
    }

    /// Parse a unit from a string abbreviation.
    /// Returns None if not recognized.
    pub fn from_str(s: &str) -> Option<Unit> {
        match s {
            // Length
            "m" | "meter" | "meters" => Some(Unit::Meter),
            "km" | "kilometer" | "kilometers" | "kilometre" | "kilometres" => Some(Unit::Kilometer),
            "cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => Some(Unit::Centimeter),
            "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => Some(Unit::Millimeter),
            "in" | "inch" | "inches" => Some(Unit::Inch),
            "ft" | "foot" | "feet" => Some(Unit::Foot),
            "yd" | "yard" | "yards" => Some(Unit::Yard),
            "mi" | "mile" | "miles" => Some(Unit::Mile),
            // Mass
            "kg" | "kilogram" | "kilograms" => Some(Unit::Kilogram),
            "g" | "gram" | "grams" => Some(Unit::Gram),
            "mg" | "milligram" | "milligrams" => Some(Unit::Milligram),
            "lb" | "lbs" | "pound" | "pounds" => Some(Unit::Pound),
            "oz" | "ounce" | "ounces" => Some(Unit::Ounce),
            // Temperature (case-sensitive variants handled below)
            "C" | "°C" | "celsius" | "Celsius" => Some(Unit::Celsius),
            "F" | "°F" | "fahrenheit" | "Fahrenheit" => Some(Unit::Fahrenheit),
            "K" | "kelvin" | "Kelvin" => Some(Unit::Kelvin),
            // Data
            "B" | "byte" | "bytes" => Some(Unit::Byte),
            "KB" | "kilobyte" | "kilobytes" => Some(Unit::Kilobyte),
            "MB" | "megabyte" | "megabytes" => Some(Unit::Megabyte),
            "GB" | "gigabyte" | "gigabytes" => Some(Unit::Gigabyte),
            "TB" | "terabyte" | "terabytes" => Some(Unit::Terabyte),
            // Time
            "s" | "sec" | "secs" | "second" | "seconds" => Some(Unit::Second),
            "min" | "mins" | "minute" | "minutes" => Some(Unit::Minute),
            "h" | "hr" | "hrs" | "hour" | "hours" => Some(Unit::Hour),
            "d" | "day" | "days" => Some(Unit::Day),
            _ => None,
        }
    }

    /// Convert `value` from `from` unit to `to` unit.
    /// Both must be in the same category.
    pub fn convert(value: f64, from: &Unit, to: &Unit) -> Result<f64> {
        if from == to {
            return Ok(value);
        }
        if from.category() != to.category() {
            return Err(CalcError::eval(format!(
                "Cannot convert {} to {}: incompatible units",
                from, to
            )));
        }

        // Temperature uses special formulas
        if from.category() == UnitCategory::Temperature {
            return convert_temperature(value, from, to);
        }

        // For all other categories: convert to base unit first, then to target
        let base = to_base(value, from);
        Ok(from_base(base, to))
    }
}

/// Convert a value in `unit` to its category's base unit.
/// Base units:
///   Length   → Meter
///   Mass     → Gram
///   Data     → Byte
///   Time     → Second
fn to_base(value: f64, unit: &Unit) -> f64 {
    match unit {
        // Length → Meter
        Unit::Meter => value,
        Unit::Kilometer => value * 1_000.0,
        Unit::Centimeter => value / 100.0,
        Unit::Millimeter => value / 1_000.0,
        Unit::Inch => value * 0.0254,
        Unit::Foot => value * 0.3048,
        Unit::Yard => value * 0.9144,
        Unit::Mile => value * 1_609.344,
        // Mass → Gram
        Unit::Gram => value,
        Unit::Kilogram => value * 1_000.0,
        Unit::Milligram => value / 1_000.0,
        Unit::Pound => value * 453.592_37,
        Unit::Ounce => value * 28.349_523_125,
        // Data → Byte
        Unit::Byte => value,
        Unit::Kilobyte => value * 1_024.0,
        Unit::Megabyte => value * 1_024.0 * 1_024.0,
        Unit::Gigabyte => value * 1_024.0 * 1_024.0 * 1_024.0,
        Unit::Terabyte => value * 1_024.0 * 1_024.0 * 1_024.0 * 1_024.0,
        // Time → Second
        Unit::Second => value,
        Unit::Minute => value * 60.0,
        Unit::Hour => value * 3_600.0,
        Unit::Day => value * 86_400.0,
        // Temperature handled separately
        Unit::Celsius | Unit::Fahrenheit | Unit::Kelvin => value,
        // Custom units are handled in the evaluator, not here
        Unit::Custom(_) => value,
    }
}

/// Convert a value in base unit to `unit`.
fn from_base(value: f64, unit: &Unit) -> f64 {
    match unit {
        // Length ← Meter
        Unit::Meter => value,
        Unit::Kilometer => value / 1_000.0,
        Unit::Centimeter => value * 100.0,
        Unit::Millimeter => value * 1_000.0,
        Unit::Inch => value / 0.0254,
        Unit::Foot => value / 0.3048,
        Unit::Yard => value / 0.9144,
        Unit::Mile => value / 1_609.344,
        // Mass ← Gram
        Unit::Gram => value,
        Unit::Kilogram => value / 1_000.0,
        Unit::Milligram => value * 1_000.0,
        Unit::Pound => value / 453.592_37,
        Unit::Ounce => value / 28.349_523_125,
        // Data ← Byte
        Unit::Byte => value,
        Unit::Kilobyte => value / 1_024.0,
        Unit::Megabyte => value / (1_024.0 * 1_024.0),
        Unit::Gigabyte => value / (1_024.0 * 1_024.0 * 1_024.0),
        Unit::Terabyte => value / (1_024.0 * 1_024.0 * 1_024.0 * 1_024.0),
        // Time ← Second
        Unit::Second => value,
        Unit::Minute => value / 60.0,
        Unit::Hour => value / 3_600.0,
        Unit::Day => value / 86_400.0,
        // Temperature handled separately
        Unit::Celsius | Unit::Fahrenheit | Unit::Kelvin => value,
        // Custom units are handled in the evaluator, not here
        Unit::Custom(_) => value,
    }
}

fn convert_temperature(value: f64, from: &Unit, to: &Unit) -> Result<f64> {
    // First convert to Kelvin
    let kelvin = match from {
        Unit::Celsius => value + 273.15,
        Unit::Fahrenheit => (value + 459.67) * 5.0 / 9.0,
        Unit::Kelvin => value,
        _ => unreachable!(),
    };
    // Then convert from Kelvin to target
    let result = match to {
        Unit::Celsius => kelvin - 273.15,
        Unit::Fahrenheit => kelvin * 9.0 / 5.0 - 459.67,
        Unit::Kelvin => kelvin,
        _ => unreachable!(),
    };
    Ok(result)
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Unit::Meter => "m",
            Unit::Kilometer => "km",
            Unit::Centimeter => "cm",
            Unit::Millimeter => "mm",
            Unit::Inch => "in",
            Unit::Foot => "ft",
            Unit::Yard => "yd",
            Unit::Mile => "mi",
            Unit::Kilogram => "kg",
            Unit::Gram => "g",
            Unit::Milligram => "mg",
            Unit::Pound => "lb",
            Unit::Ounce => "oz",
            Unit::Celsius => "°C",
            Unit::Fahrenheit => "°F",
            Unit::Kelvin => "K",
            Unit::Byte => "B",
            Unit::Kilobyte => "KB",
            Unit::Megabyte => "MB",
            Unit::Gigabyte => "GB",
            Unit::Terabyte => "TB",
            Unit::Second => "s",
            Unit::Minute => "min",
            Unit::Hour => "h",
            Unit::Day => "d",
            Unit::Custom(ref name) => return write!(f, "{}", name),
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_conversion() {
        let result = Unit::convert(1.0, &Unit::Kilometer, &Unit::Meter).unwrap();
        assert!((result - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn test_temperature_conversion() {
        let result = Unit::convert(100.0, &Unit::Celsius, &Unit::Fahrenheit).unwrap();
        assert!((result - 212.0).abs() < 1e-6);

        let result = Unit::convert(0.0, &Unit::Celsius, &Unit::Kelvin).unwrap();
        assert!((result - 273.15).abs() < 1e-6);
    }

    #[test]
    fn test_incompatible_units() {
        let result = Unit::convert(1.0, &Unit::Meter, &Unit::Gram);
        assert!(result.is_err());
    }
}
