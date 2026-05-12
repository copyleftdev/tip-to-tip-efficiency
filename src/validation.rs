use crate::{config::TipToTipConfig, error::TipToTipError, profile::D2fProfile};

pub(crate) fn validate_config(config: TipToTipConfig) -> Result<(), TipToTipError> {
    validate_non_negative("girth_tolerance", config.girth_tolerance)?;
    validate_non_negative("t2o_tolerance_seconds", config.t2o_tolerance_seconds)?;
    if config.fallback_lanes == 0 {
        return Err(TipToTipError::InvalidLaneCount);
    }
    Ok(())
}

pub(crate) fn validate_profile(profile: D2fProfile) -> Result<(), TipToTipError> {
    validate_non_negative("d2f", profile.d2f)?;
    validate_non_negative("length", profile.length)?;
    validate_non_negative("girth", profile.girth)?;
    validate_non_negative("t2o", profile.t2o)?;
    Ok(())
}

pub(crate) fn validate_non_negative(field: &'static str, value: f64) -> Result<(), TipToTipError> {
    if !value.is_finite() || value < 0.0 {
        return Err(TipToTipError::InvalidNumber { field, value });
    }
    Ok(())
}
