use std::fmt;

/// Errors returned by the tip-to-tip planner.
#[derive(Debug, Clone, PartialEq)]
pub enum TipToTipError {
    /// A numeric field was negative, infinite, or NaN.
    InvalidNumber {
        /// Name of the invalid field.
        field: &'static str,
        /// Rejected value.
        value: f64,
    },
    /// A lane count was zero.
    InvalidLaneCount,
}

impl fmt::Display for TipToTipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber { field, value } => {
                write!(f, "{field} must be finite and non-negative, got {value}")
            }
            Self::InvalidLaneCount => write!(f, "lane count must be greater than zero"),
        }
    }
}

impl std::error::Error for TipToTipError {}
