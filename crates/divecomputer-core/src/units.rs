//! Units of measurement used by the dive model
//! ([spec](https://michidk.github.io/divecomputer-rs/spec/conventions.html)).
//!
//! Each quantity is a newtype over `f64` in one fixed SI-style unit, so that parsers
//! convert device units exactly once. With the `serde` feature each serializes as a
//! plain number in that unit.

/// Metres per foot.
const METRES_PER_FOOT: f64 = 0.3048;
/// Bar per psi.
const BAR_PER_PSI: f64 = 0.068_947_572_9;

macro_rules! unit {
    ($(#[$doc:meta])* $name:ident, $from:ident, $get:ident, $unit:literal) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize), serde(transparent))]
        pub struct $name(f64);

        impl $name {
            #[doc = concat!("A value in ", $unit, ".")]
            #[must_use]
            pub const fn $from(value: f64) -> Self {
                Self(value)
            }

            #[doc = concat!("The value in ", $unit, ".")]
            #[must_use]
            pub const fn $get(self) -> f64 {
                self.0
            }
        }
    };
}

unit!(
    /// A depth below the surface, in metres.
    Depth,
    from_metres,
    metres,
    "metres"
);
unit!(
    /// A temperature, in degrees Celsius.
    Temperature,
    from_celsius,
    celsius,
    "degrees Celsius"
);
unit!(
    /// A pressure (tank, surface or partial pressure), in bar.
    Pressure,
    from_bar,
    bar,
    "bar"
);
unit!(
    /// The share of one gas in a breathing mix, in percent (0–100).
    GasFraction,
    from_percent,
    percent,
    "percent"
);

impl Depth {
    /// A depth in feet.
    #[must_use]
    pub const fn from_feet(feet: f64) -> Self {
        Self(feet * METRES_PER_FOOT)
    }
}

impl Temperature {
    /// A temperature in degrees Fahrenheit.
    #[must_use]
    pub const fn from_fahrenheit(fahrenheit: f64) -> Self {
        Self((fahrenheit - 32.0) * 5.0 / 9.0)
    }
}

impl Pressure {
    /// A pressure in millibar.
    #[must_use]
    pub const fn from_millibar(millibar: f64) -> Self {
        Self(millibar / 1000.0)
    }

    /// A pressure in pounds per square inch.
    #[must_use]
    pub const fn from_psi(psi: f64) -> Self {
        Self(psi * BAR_PER_PSI)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_imperial_units() {
        assert!((Depth::from_feet(10.0).metres() - 3.048).abs() < 1e-9);
        assert!((Temperature::from_fahrenheit(212.0).celsius() - 100.0).abs() < 1e-9);
        assert!((Pressure::from_psi(14.503_773_8).bar() - 1.0).abs() < 1e-6);
        assert!((Pressure::from_millibar(1013.0).bar() - 1.013).abs() < 1e-9);
    }
}
