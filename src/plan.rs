/// A feasible adjacent D2F pairing.
#[derive(Debug, Clone, PartialEq)]
pub struct TipToTipPair {
    /// The two profile identifiers in D2F-sorted order.
    pub ids: [usize; 2],
    /// Absolute D2F distance between the two profiles.
    pub d2f_gap: f64,
    /// Combined reach length available to bridge the D2F gap.
    pub bridge: f64,
    /// Angle implied by `asin(d2f_gap / bridge)`, clamped to the valid domain.
    pub theta_d_radians: f64,
    /// Absolute girth difference.
    pub girth_gap: f64,
    /// Absolute T2O difference.
    pub t2o_gap_seconds: f64,
    /// Sum of both participants' T2O values.
    pub work_seconds: f64,
    /// Wall-clock time consumed by this pair.
    pub duration_seconds: f64,
    /// Idle time inside the pair when one participant finishes first.
    pub wasted_seconds: f64,
}

/// One round of up to two pairs, meaning up to four simultaneous participants.
#[derive(Debug, Clone, PartialEq)]
pub struct TipToTipCohort {
    /// Indices into [`TipToTipPlan::pairs`] scheduled in this cohort.
    pub pair_indices: Vec<usize>,
    /// Number of participants active in the cohort.
    pub participant_count: usize,
    /// Wall-clock time consumed by the cohort.
    pub duration_seconds: f64,
    /// Idle time introduced by cohort-level pair imbalance.
    pub wasted_seconds: f64,
}

/// Schedule and metrics for the tip-to-tip model.
#[derive(Debug, Clone, PartialEq)]
pub struct TipToTipPlan {
    /// Number of input profiles.
    pub total_participants: usize,
    /// Profile identifiers sorted by ascending D2F.
    pub sorted_ids_by_d2f: Vec<usize>,
    /// Feasible adjacent D2F pairs.
    pub pairs: Vec<TipToTipPair>,
    /// Four-lane rounds built from up to two pairs each.
    pub cohorts: Vec<TipToTipCohort>,
    /// Profile identifiers that could not enter a feasible tip-to-tip pair.
    pub unmatched_ids: Vec<usize>,
    /// Seconds spent in the four-lane tip-to-tip schedule.
    pub tip_to_tip_seconds: f64,
    /// Seconds spent handling unmatched profiles through fallback lanes.
    pub fallback_seconds: f64,
    /// Total modeled wall-clock seconds.
    pub total_seconds: f64,
    /// Two-lane closed-form baseline using the input mean T2O.
    pub two_lane_baseline_seconds: f64,
    /// Four-lane ideal closed-form baseline using the input mean T2O.
    pub ideal_four_lane_seconds: f64,
    /// `two_lane_baseline_seconds / total_seconds`.
    pub throughput_gain_over_two_lane: f64,
    /// Pair-level plus cohort-level idle seconds.
    pub wasted_seconds: f64,
}
