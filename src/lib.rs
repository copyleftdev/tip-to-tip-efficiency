//! A precise, intentionally ridiculous model of tip-to-tip efficiency.
//!
//! This crate implements the optimization joke from *Silicon Valley* as a
//! small, deterministic scheduling model. It does not claim physical realism.
//! It does preserve the useful engineering shape of the scene:
//!
//! - Dinesh's closed form: total work divided by simultaneous lanes.
//! - Gilfoyle's refinement: sort by D2F, pair compatible profiles, compute
//!   `theta_d`, schedule four-at-a-time cohorts, and account for hot swapping.
//! - Enterprise escalation: audit many rooms with policy thresholds and portfolio
//!   metrics.
//! - Acceleration advice: keep normal rooms on CPU and identify GPU-scale
//!   portfolio workloads explicitly.
//!
//! ```rust
//! use tip_to_tip::{D2fProfile, TipToTip, TipToTipConfig};
//!
//! let profiles = [
//!     D2fProfile::new(1, 10.0, 2.0, 4.0, 10.0),
//!     D2fProfile::new(2, 11.0, 2.0, 4.0, 10.0),
//!     D2fProfile::new(3, 12.0, 2.0, 4.0, 10.0),
//!     D2fProfile::new(4, 13.0, 2.0, 4.0, 10.0),
//! ];
//!
//! let plan = TipToTip::plan(&profiles, TipToTipConfig::default())?;
//! assert_eq!(plan.total_seconds, 10.0);
//! assert_eq!(plan.throughput_gain_over_two_lane, 2.0);
//! # Ok::<(), tip_to_tip::TipToTipError>(())
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod acceleration;
mod config;
mod constants;
mod enterprise;
mod error;
mod plan;
mod planner;
mod profile;
mod tip_to_tip;
mod validation;

pub use acceleration::{
    ComputeBackend, GpuOffloadPhase, TipToTipAcceleration, TipToTipAccelerationReport,
    WorkloadShape,
};
pub use config::TipToTipConfig;
pub use enterprise::{
    EmpireSignal, TipToTipEmpire, TipToTipEmpirePolicy, TipToTipEmpireReport, TipToTipRoom,
    TipToTipRoomReport,
};
pub use error::TipToTipError;
pub use plan::{TipToTipCohort, TipToTipPair, TipToTipPlan};
pub use profile::D2fProfile;
pub use tip_to_tip::TipToTip;
