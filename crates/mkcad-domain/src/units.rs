//! Unit system with dimension kinds, conversions, and quantities.
//!
//! Supports length, angle, and mass dimensions with standard SI and imperial units.
//! All internal storage uses SI-like base units (mm, rad, kg).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dimension kind: length, angle, mass, or unitless.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum DimensionKind {
    /// Length dimension (base unit: mm).
    Length,
    /// Angle dimension (base unit: rad).
    Angle,
    /// Mass dimension (base unit: kg).
    Mass,
    /// Unitless (dimensionless).
    Unitless,
}

impl DimensionKind {
    /// Returns the SI-like base unit symbol for this dimension.
    pub fn base_unit(&self) -> &'static str {
        match self {
            DimensionKind::Length => "mm",
            DimensionKind::Angle => "rad",
            DimensionKind::Mass => "kg",
            DimensionKind::Unitless => "",
        }
    }
}

/// A physical quantity: value in SI-like base units + dimension kind.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    /// Value in base units (mm for length, rad for angle, kg for mass).
    pub value: f64,
    /// Dimension of this quantity.
    pub dimension: DimensionKind,
}

impl Quantity {
    /// Creates a dimensionless quantity.
    pub fn unitless(value: f64) -> Self {
        Quantity { value, dimension: DimensionKind::Unitless }
    }

    /// Creates a length quantity (in mm).
    pub fn length_mm(value: f64) -> Self {
        Quantity { value, dimension: DimensionKind::Length }
    }

    /// Creates an angle quantity (in rad).
    pub fn angle_rad(value: f64) -> Self {
        Quantity { value, dimension: DimensionKind::Angle }
    }

    /// Creates a mass quantity (in kg).
    pub fn mass_kg(value: f64) -> Self {
        Quantity { value, dimension: DimensionKind::Mass }
    }

    /// Checks if two quantities are dimensionally compatible.
    pub fn compatible_with(&self, other: Quantity) -> bool {
        self.dimension == other.dimension
    }

    /// Returns a string representation with base units.
    pub fn to_string_base(&self) -> String {
        let unit = self.dimension.base_unit();
        if unit.is_empty() {
            format!("{}", self.value)
        } else {
            format!("{} {}", self.value, unit)
        }
    }
}

/// Unit table with conversion factors to base units.
pub struct UnitTable {
    /// Map from unit symbol to (dimension kind, conversion factor to base).
    units: HashMap<String, (DimensionKind, f64)>,
}

impl UnitTable {
    /// Creates a standard unit table.
    pub fn new() -> Self {
        let mut units = HashMap::new();

        // Length units (base: mm)
        units.insert("mm".to_string(), (DimensionKind::Length, 1.0));
        units.insert("cm".to_string(), (DimensionKind::Length, 10.0));
        units.insert("m".to_string(), (DimensionKind::Length, 1000.0));
        units.insert("in".to_string(), (DimensionKind::Length, 25.4)); // inch to mm
        units.insert("ft".to_string(), (DimensionKind::Length, 304.8)); // foot to mm

        // Angle units (base: rad)
        units.insert("rad".to_string(), (DimensionKind::Angle, 1.0));
        units.insert("deg".to_string(), (DimensionKind::Angle, std::f64::consts::PI / 180.0));

        // Mass units (base: kg)
        units.insert("kg".to_string(), (DimensionKind::Mass, 1.0));
        units.insert("g".to_string(), (DimensionKind::Mass, 0.001));
        units.insert("lb".to_string(), (DimensionKind::Mass, 0.453592)); // pound to kg

        // Reserved SI unit symbols (not implemented yet; reserved per F54)
        units.insert("W".to_string(), (DimensionKind::Unitless, 1.0)); // watt
        units.insert("A".to_string(), (DimensionKind::Unitless, 1.0)); // ampere

        UnitTable { units }
    }

    /// Parses a unit symbol and returns (dimension, conversion factor).
    pub fn parse(&self, unit: &str) -> Option<(DimensionKind, f64)> {
        self.units.get(unit).copied()
    }

    /// Checks if a unit symbol is reserved.
    pub fn is_reserved(&self, symbol: &str) -> bool {
        self.units.contains_key(symbol)
    }

    /// Returns all reserved unit symbols.
    pub fn reserved_units(&self) -> Vec<&str> {
        self.units.keys().map(|s| s.as_str()).collect()
    }

    /// Converts a value from one unit to another.
    /// Returns None if units are incompatible.
    pub fn convert(&self, value: f64, from: &str, to: &str) -> Option<f64> {
        let (from_dim, from_factor) = self.parse(from)?;
        let (to_dim, to_factor) = self.parse(to)?;

        if from_dim != to_dim {
            return None;
        }

        // Convert: value (in "from") -> base -> "to"
        Some((value * from_factor) / to_factor)
    }
}

impl Default for UnitTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension_kind_base_unit() {
        assert_eq!(DimensionKind::Length.base_unit(), "mm");
        assert_eq!(DimensionKind::Angle.base_unit(), "rad");
        assert_eq!(DimensionKind::Mass.base_unit(), "kg");
        assert_eq!(DimensionKind::Unitless.base_unit(), "");
    }

    #[test]
    fn test_quantity_creation() {
        let len = Quantity::length_mm(40.0);
        assert_eq!(len.value, 40.0);
        assert_eq!(len.dimension, DimensionKind::Length);

        let unitless = Quantity::unitless(42.0);
        assert_eq!(unitless.dimension, DimensionKind::Unitless);
    }

    #[test]
    fn test_quantity_compatible() {
        let len1 = Quantity::length_mm(40.0);
        let len2 = Quantity::length_mm(50.0);
        assert!(len1.compatible_with(len2));

        let unitless = Quantity::unitless(1.0);
        assert!(!len1.compatible_with(unitless));
    }

    #[test]
    fn test_unit_table_parse() {
        let table = UnitTable::new();
        assert!(table.parse("mm").is_some());
        assert!(table.parse("in").is_some());
        assert!(table.parse("rad").is_some());
        assert!(table.parse("invalid").is_none());
    }

    #[test]
    fn test_unit_table_reserved() {
        let table = UnitTable::new();
        assert!(table.is_reserved("mm"));
        assert!(table.is_reserved("in"));
        assert!(!table.is_reserved("Len"));
    }

    #[test]
    fn test_unit_conversion_length() {
        let table = UnitTable::new();
        // 1 inch = 25.4 mm
        let result = table.convert(1.0, "in", "mm").unwrap();
        assert!((result - 25.4).abs() < 1e-10);

        // 40 mm = 40 mm
        let result = table.convert(40.0, "mm", "mm").unwrap();
        assert_eq!(result, 40.0);

        // 1 m = 1000 mm
        let result = table.convert(1.0, "m", "mm").unwrap();
        assert_eq!(result, 1000.0);
    }

    #[test]
    fn test_unit_conversion_incompatible() {
        let table = UnitTable::new();
        // Can't convert length to angle
        assert!(table.convert(1.0, "mm", "rad").is_none());
    }

    #[test]
    fn test_unit_conversion_angle() {
        let table = UnitTable::new();
        // 180 degrees = pi radians
        let result = table.convert(180.0, "deg", "rad").unwrap();
        assert!((result - std::f64::consts::PI).abs() < 1e-10);
    }
}
