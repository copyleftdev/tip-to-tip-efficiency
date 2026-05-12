/// One participant's parameters from the episode's tip-to-tip model.
///
/// `d2f` and `t2o` intentionally keep the scene's shorthand. Units are caller
/// chosen, but all distance fields must use the same unit and `t2o` is seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct D2fProfile {
    /// Stable caller-provided identifier carried through the schedule.
    pub id: usize,
    /// Distance-to-floor.
    pub d2f: f64,
    /// Reach length used to bridge D2F gaps.
    pub length: f64,
    /// Girth value used for compatibility grouping.
    pub girth: f64,
    /// Time-to-orgasm, in seconds.
    pub t2o: f64,
}

impl D2fProfile {
    /// Construct a participant profile.
    pub const fn new(id: usize, d2f: f64, length: f64, girth: f64, t2o: f64) -> Self {
        Self {
            id,
            d2f,
            length,
            girth,
            t2o,
        }
    }
}
