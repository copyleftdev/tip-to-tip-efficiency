/// Tuning knobs for the D2F pairing model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TipToTipConfig {
    /// Maximum permitted absolute girth difference inside a pair.
    pub girth_tolerance: f64,
    /// Maximum permitted absolute T2O difference without hot swapping.
    pub t2o_tolerance_seconds: f64,
    /// When enabled, pair duration is average work time rather than the slower
    /// participant's T2O.
    pub allow_hot_swap: bool,
    /// Number of lanes used for profiles that cannot join the four-lane model.
    pub fallback_lanes: usize,
}

impl Default for TipToTipConfig {
    fn default() -> Self {
        Self {
            girth_tolerance: 0.5,
            t2o_tolerance_seconds: 1.0,
            allow_hot_swap: false,
            fallback_lanes: 2,
        }
    }
}
