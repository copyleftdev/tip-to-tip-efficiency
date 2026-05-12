use crate::{
    config::TipToTipConfig, error::TipToTipError, plan::TipToTipPlan, planner, profile::D2fProfile,
};

/// Implementation of the episode's "tip-to-tip efficiency" thought experiment.
///
/// This is the crate's public ingress point for core room-level math. Supporting
/// modules own validation, scheduling, output types, and enterprise reporting.
#[derive(Debug, Clone, Copy, Default)]
pub struct TipToTip;

impl TipToTip {
    /// Dinesh's room-level estimate: `participants * mean_time / lanes`.
    pub fn dinesh_closed_form(
        participants: usize,
        mean_time_seconds: f64,
        simultaneous_lanes: usize,
    ) -> Result<f64, TipToTipError> {
        planner::dinesh_closed_form(participants, mean_time_seconds, simultaneous_lanes)
    }

    /// The pure four-at-a-time estimate used by the scene's initial insight.
    pub fn ideal_middle_out_seconds(
        participants: usize,
        mean_time_seconds: f64,
    ) -> Result<f64, TipToTipError> {
        planner::ideal_middle_out_seconds(participants, mean_time_seconds)
    }

    /// Sort by D2F, build adjacent feasible pairs, then schedule two pairs per
    /// cohort. Incompatible or odd participants fall back to the configured lane
    /// count.
    pub fn plan(
        profiles: &[D2fProfile],
        config: TipToTipConfig,
    ) -> Result<TipToTipPlan, TipToTipError> {
        planner::plan(profiles, config)
    }
}
