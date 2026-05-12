use crate::{
    config::TipToTipConfig, constants::EPSILON, error::TipToTipError, plan::TipToTipPlan,
    profile::D2fProfile, tip_to_tip::TipToTip, validation::validate_non_negative,
};

/// A named planning unit for enterprise-level audit reports.
#[derive(Debug, Clone, PartialEq)]
pub struct TipToTipRoom {
    /// Human-readable room, team, or deployment name.
    pub name: String,
    /// Profiles assigned to this room.
    pub profiles: Vec<D2fProfile>,
    /// Planner configuration for this room.
    pub config: TipToTipConfig,
}

impl TipToTipRoom {
    /// Construct a room with its own profiles and planner configuration.
    pub fn new(
        name: impl Into<String>,
        profiles: impl Into<Vec<D2fProfile>>,
        config: TipToTipConfig,
    ) -> Self {
        Self {
            name: name.into(),
            profiles: profiles.into(),
            config,
        }
    }
}

/// Governance thresholds for the enterprise audit layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TipToTipEmpirePolicy {
    /// Minimum acceptable room-level gain over the two-lane baseline.
    pub minimum_gain_over_two_lane: f64,
    /// Maximum acceptable `wasted_seconds / total_seconds`.
    pub maximum_waste_ratio: f64,
    /// Maximum acceptable `unmatched_count / total_participants`.
    pub maximum_unmatched_ratio: f64,
}

impl Default for TipToTipEmpirePolicy {
    fn default() -> Self {
        Self {
            minimum_gain_over_two_lane: 1.25,
            maximum_waste_ratio: 0.10,
            maximum_unmatched_ratio: 0.0,
        }
    }
}

/// Audit signal emitted by the enterprise layer.
#[derive(Debug, Clone, PartialEq)]
pub enum EmpireSignal {
    /// The room meets every configured policy threshold.
    OnPlan,
    /// The room has no participants.
    EmptyRoom,
    /// Room gain is below policy.
    BelowMinimumGain {
        /// Actual gain over the two-lane baseline.
        actual: f64,
        /// Required gain.
        minimum: f64,
    },
    /// Room idle time exceeds policy.
    WasteBudgetExceeded {
        /// Actual waste ratio.
        actual: f64,
        /// Maximum allowed waste ratio.
        maximum: f64,
    },
    /// Too many profiles fell back outside the four-lane model.
    UnmatchedBudgetExceeded {
        /// Actual unmatched ratio.
        actual: f64,
        /// Maximum allowed unmatched ratio.
        maximum: f64,
    },
}

/// Enterprise audit output for one room.
#[derive(Debug, Clone, PartialEq)]
pub struct TipToTipRoomReport {
    /// Room name.
    pub name: String,
    /// Core plan generated for the room.
    pub plan: TipToTipPlan,
    /// Policy signals for the room.
    pub signals: Vec<EmpireSignal>,
    /// True when the room meets all configured thresholds.
    pub promoted: bool,
}

/// Enterprise rollup across many rooms.
#[derive(Debug, Clone, PartialEq)]
pub struct TipToTipEmpireReport {
    /// Per-room reports.
    pub rooms: Vec<TipToTipRoomReport>,
    /// Total participants across all rooms.
    pub total_participants: usize,
    /// Total modeled wall-clock seconds across all rooms.
    pub total_seconds: f64,
    /// Aggregate two-lane baseline seconds.
    pub two_lane_baseline_seconds: f64,
    /// Aggregate ideal four-lane seconds.
    pub ideal_four_lane_seconds: f64,
    /// Aggregate gain over the two-lane baseline.
    pub throughput_gain_over_two_lane: f64,
    /// Aggregate idle seconds.
    pub wasted_seconds: f64,
    /// Total unmatched profiles.
    pub unmatched_count: usize,
    /// Number of rooms that passed all policy thresholds.
    pub promoted_rooms: usize,
    /// Room with the largest `total_seconds`, if any rooms were audited.
    pub bottleneck_room: Option<String>,
}

/// Enterprise facade for portfolio-level tip-to-tip governance.
#[derive(Debug, Clone, Copy, Default)]
pub struct TipToTipEmpire;

impl TipToTipEmpire {
    /// Audit many rooms, applying policy thresholds and aggregate metrics.
    pub fn audit(
        rooms: &[TipToTipRoom],
        policy: TipToTipEmpirePolicy,
    ) -> Result<TipToTipEmpireReport, TipToTipError> {
        validate_policy(policy)?;

        let mut room_reports = Vec::with_capacity(rooms.len());
        let mut total_participants = 0;
        let mut total_seconds = 0.0;
        let mut two_lane_baseline_seconds = 0.0;
        let mut ideal_four_lane_seconds = 0.0;
        let mut wasted_seconds = 0.0;
        let mut unmatched_count = 0;
        let mut promoted_rooms = 0;
        let mut bottleneck_room = None::<String>;
        let mut bottleneck_seconds = 0.0;

        for room in rooms {
            let plan = TipToTip::plan(&room.profiles, room.config)?;
            let signals = audit_room(&plan, policy);
            let promoted = signals == [EmpireSignal::OnPlan];

            total_participants += plan.total_participants;
            total_seconds += plan.total_seconds;
            two_lane_baseline_seconds += plan.two_lane_baseline_seconds;
            ideal_four_lane_seconds += plan.ideal_four_lane_seconds;
            wasted_seconds += plan.wasted_seconds;
            unmatched_count += plan.unmatched_ids.len();
            promoted_rooms += usize::from(promoted);

            let replaces_bottleneck = match &bottleneck_room {
                Some(name) => plan
                    .total_seconds
                    .total_cmp(&bottleneck_seconds)
                    .then_with(|| room.name.as_str().cmp(name.as_str()))
                    .is_gt(),
                None => true,
            };
            if replaces_bottleneck {
                bottleneck_seconds = plan.total_seconds;
                bottleneck_room = Some(room.name.clone());
            }

            room_reports.push(TipToTipRoomReport {
                name: room.name.clone(),
                plan,
                signals,
                promoted,
            });
        }

        let throughput_gain_over_two_lane = if total_seconds <= EPSILON {
            1.0
        } else {
            two_lane_baseline_seconds / total_seconds
        };

        Ok(TipToTipEmpireReport {
            rooms: room_reports,
            total_participants,
            total_seconds,
            two_lane_baseline_seconds,
            ideal_four_lane_seconds,
            throughput_gain_over_two_lane,
            wasted_seconds,
            unmatched_count,
            promoted_rooms,
            bottleneck_room,
        })
    }
}

fn audit_room(plan: &TipToTipPlan, policy: TipToTipEmpirePolicy) -> Vec<EmpireSignal> {
    let mut signals = Vec::with_capacity(4);

    if plan.total_participants == 0 {
        signals.push(EmpireSignal::EmptyRoom);
    }

    if plan.throughput_gain_over_two_lane + EPSILON < policy.minimum_gain_over_two_lane {
        signals.push(EmpireSignal::BelowMinimumGain {
            actual: plan.throughput_gain_over_two_lane,
            minimum: policy.minimum_gain_over_two_lane,
        });
    }

    let waste_ratio = if plan.total_seconds <= EPSILON {
        0.0
    } else {
        plan.wasted_seconds / plan.total_seconds
    };
    if waste_ratio > policy.maximum_waste_ratio + EPSILON {
        signals.push(EmpireSignal::WasteBudgetExceeded {
            actual: waste_ratio,
            maximum: policy.maximum_waste_ratio,
        });
    }

    let unmatched_ratio = if plan.total_participants == 0 {
        0.0
    } else {
        plan.unmatched_ids.len() as f64 / plan.total_participants as f64
    };
    if unmatched_ratio > policy.maximum_unmatched_ratio + EPSILON {
        signals.push(EmpireSignal::UnmatchedBudgetExceeded {
            actual: unmatched_ratio,
            maximum: policy.maximum_unmatched_ratio,
        });
    }

    if signals.is_empty() {
        signals.push(EmpireSignal::OnPlan);
    }

    signals
}

fn validate_policy(policy: TipToTipEmpirePolicy) -> Result<(), TipToTipError> {
    validate_non_negative(
        "minimum_gain_over_two_lane",
        policy.minimum_gain_over_two_lane,
    )?;
    validate_non_negative("maximum_waste_ratio", policy.maximum_waste_ratio)?;
    validate_non_negative("maximum_unmatched_ratio", policy.maximum_unmatched_ratio)?;
    Ok(())
}
