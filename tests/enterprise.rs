use tip_to_tip::{
    D2fProfile, EmpireSignal, TipToTipConfig, TipToTipEmpire, TipToTipEmpirePolicy, TipToTipError,
    TipToTipRoom,
};

const EPSILON: f64 = 1.0e-9;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {actual} to be within {EPSILON} of {expected}"
    );
}

fn room(name: &str, t2o: f64) -> TipToTipRoom {
    TipToTipRoom::new(
        name,
        [
            D2fProfile::new(1, 0.0, 2.0, 4.0, t2o),
            D2fProfile::new(2, 1.0, 2.0, 4.0, t2o),
            D2fProfile::new(3, 2.0, 2.0, 4.0, t2o),
            D2fProfile::new(4, 3.0, 2.0, 4.0, t2o),
        ],
        TipToTipConfig::default(),
    )
}

fn wasteful_room() -> TipToTipRoom {
    TipToTipRoom::new(
        "wasteful",
        [
            D2fProfile::new(1, 0.0, 2.0, 4.0, 10.0),
            D2fProfile::new(2, 1.0, 2.0, 4.0, 20.0),
        ],
        TipToTipConfig {
            t2o_tolerance_seconds: 10.0,
            ..TipToTipConfig::default()
        },
    )
}

fn incompatible_room() -> TipToTipRoom {
    TipToTipRoom::new(
        "fallback",
        [
            D2fProfile::new(1, 0.0, 1.0, 4.0, 10.0),
            D2fProfile::new(2, 10.0, 1.0, 4.0, 10.0),
        ],
        TipToTipConfig::default(),
    )
}

#[test]
fn empire_rolls_up_many_rooms() {
    let rooms = [room("alpha", 10.0), room("bravo", 20.0)];

    let report = TipToTipEmpire::audit(&rooms, TipToTipEmpirePolicy::default()).unwrap();

    assert_eq!(report.rooms.len(), 2);
    assert_eq!(report.total_participants, 8);
    assert_close(report.total_seconds, 30.0);
    assert_close(report.two_lane_baseline_seconds, 60.0);
    assert_close(report.ideal_four_lane_seconds, 30.0);
    assert_close(report.throughput_gain_over_two_lane, 2.0);
    assert_close(report.wasted_seconds, 0.0);
    assert_eq!(report.unmatched_count, 0);
    assert_eq!(report.promoted_rooms, 2);
    assert_eq!(report.bottleneck_room.as_deref(), Some("bravo"));
}

#[test]
fn bottleneck_prefers_lexicographically_largest_name_on_equal_duration() {
    let rooms = [
        room("zulu", 10.0),
        room("alpha", 10.0),
        room("middle", 10.0),
    ];

    let report = TipToTipEmpire::audit(&rooms, TipToTipEmpirePolicy::default()).unwrap();

    assert_eq!(report.bottleneck_room.as_deref(), Some("zulu"));
}

#[test]
fn bottleneck_prefers_duration_before_name() {
    let rooms = [room("alpha", 20.0), room("zulu", 10.0)];

    let report = TipToTipEmpire::audit(&rooms, TipToTipEmpirePolicy::default()).unwrap();

    assert_eq!(report.bottleneck_room.as_deref(), Some("alpha"));
}

#[test]
fn empire_flags_empty_and_wasteful_rooms() {
    let empty = TipToTipRoom::new("empty", [], TipToTipConfig::default());
    let wasteful = wasteful_room();
    let policy = TipToTipEmpirePolicy {
        minimum_gain_over_two_lane: 0.0,
        maximum_waste_ratio: 0.10,
        maximum_unmatched_ratio: 1.0,
    };

    let report = TipToTipEmpire::audit(&[empty, wasteful], policy).unwrap();

    assert!(report.rooms[0].signals.contains(&EmpireSignal::EmptyRoom));
    assert!(!report.rooms[0].promoted);
    assert!(report.rooms[1]
        .signals
        .iter()
        .any(|signal| matches!(signal, EmpireSignal::WasteBudgetExceeded { .. })));
    assert!(!report.rooms[1].promoted);
}

#[test]
fn gain_threshold_is_epsilon_inclusive() {
    let policy = TipToTipEmpirePolicy {
        minimum_gain_over_two_lane: 2.0 + (EPSILON / 2.0),
        ..TipToTipEmpirePolicy::default()
    };

    let report = TipToTipEmpire::audit(&[room("garage", 10.0)], policy).unwrap();

    assert_eq!(report.rooms[0].signals, [EmpireSignal::OnPlan]);
    assert!(report.rooms[0].promoted);
}

#[test]
fn gain_threshold_exact_epsilon_margin_is_still_on_plan() {
    let policy = TipToTipEmpirePolicy {
        minimum_gain_over_two_lane: 2.0 + EPSILON,
        ..TipToTipEmpirePolicy::default()
    };

    let report = TipToTipEmpire::audit(&[room("garage", 10.0)], policy).unwrap();

    assert_eq!(report.rooms[0].signals, [EmpireSignal::OnPlan]);
    assert!(report.rooms[0].promoted);
}

#[test]
fn gain_threshold_beyond_epsilon_margin_is_flagged() {
    let policy = TipToTipEmpirePolicy {
        minimum_gain_over_two_lane: 2.0 + (EPSILON * 2.0),
        ..TipToTipEmpirePolicy::default()
    };

    let report = TipToTipEmpire::audit(&[room("garage", 10.0)], policy).unwrap();

    assert!(report.rooms[0]
        .signals
        .iter()
        .any(|signal| matches!(signal, EmpireSignal::BelowMinimumGain { .. })));
    assert!(!report.rooms[0].promoted);
}

#[test]
fn waste_threshold_is_epsilon_inclusive() {
    let wasteful = wasteful_room();
    let policy = TipToTipEmpirePolicy {
        minimum_gain_over_two_lane: 0.0,
        maximum_waste_ratio: 0.5 - (EPSILON / 2.0),
        maximum_unmatched_ratio: 1.0,
    };

    let report = TipToTipEmpire::audit(&[wasteful], policy).unwrap();

    assert_eq!(report.rooms[0].signals, [EmpireSignal::OnPlan]);
    assert!(report.rooms[0].promoted);
}

#[test]
fn waste_threshold_exact_epsilon_margin_is_still_on_plan() {
    let policy = TipToTipEmpirePolicy {
        minimum_gain_over_two_lane: 0.0,
        maximum_waste_ratio: 0.5 - EPSILON,
        maximum_unmatched_ratio: 1.0,
    };

    let report = TipToTipEmpire::audit(&[wasteful_room()], policy).unwrap();

    assert_eq!(report.rooms[0].signals, [EmpireSignal::OnPlan]);
    assert!(report.rooms[0].promoted);
}

#[test]
fn waste_threshold_beyond_epsilon_margin_is_flagged() {
    let policy = TipToTipEmpirePolicy {
        minimum_gain_over_two_lane: 0.0,
        maximum_waste_ratio: 0.5 - (EPSILON * 2.0),
        maximum_unmatched_ratio: 1.0,
    };

    let report = TipToTipEmpire::audit(&[wasteful_room()], policy).unwrap();

    assert!(report.rooms[0]
        .signals
        .iter()
        .any(|signal| matches!(signal, EmpireSignal::WasteBudgetExceeded { .. })));
    assert!(!report.rooms[0].promoted);
}

#[test]
fn unmatched_threshold_is_epsilon_inclusive() {
    let incompatible = incompatible_room();
    let policy = TipToTipEmpirePolicy {
        minimum_gain_over_two_lane: 0.0,
        maximum_waste_ratio: 1.0,
        maximum_unmatched_ratio: 1.0 - (EPSILON / 2.0),
    };

    let report = TipToTipEmpire::audit(&[incompatible], policy).unwrap();

    assert_eq!(report.rooms[0].signals, [EmpireSignal::OnPlan]);
    assert!(report.rooms[0].promoted);
}

#[test]
fn unmatched_threshold_exact_epsilon_margin_is_still_on_plan() {
    let policy = TipToTipEmpirePolicy {
        minimum_gain_over_two_lane: 0.0,
        maximum_waste_ratio: 1.0,
        maximum_unmatched_ratio: 1.0 - EPSILON,
    };

    let report = TipToTipEmpire::audit(&[incompatible_room()], policy).unwrap();

    assert_eq!(report.rooms[0].signals, [EmpireSignal::OnPlan]);
    assert!(report.rooms[0].promoted);
}

#[test]
fn unmatched_threshold_beyond_epsilon_margin_is_flagged() {
    let policy = TipToTipEmpirePolicy {
        minimum_gain_over_two_lane: 0.0,
        maximum_waste_ratio: 1.0,
        maximum_unmatched_ratio: 1.0 - (EPSILON * 2.0),
    };

    let report = TipToTipEmpire::audit(&[incompatible_room()], policy).unwrap();

    assert!(report.rooms[0]
        .signals
        .iter()
        .any(|signal| matches!(signal, EmpireSignal::UnmatchedBudgetExceeded { .. })));
    assert!(!report.rooms[0].promoted);
}

#[test]
fn unmatched_ratio_uses_division_not_remainder() {
    let half_unmatched = TipToTipRoom::new(
        "half-unmatched",
        [
            D2fProfile::new(1, 0.0, 2.0, 4.0, 10.0),
            D2fProfile::new(2, 1.0, 2.0, 4.0, 10.0),
            D2fProfile::new(3, 20.0, 1.0, 4.0, 10.0),
            D2fProfile::new(4, 30.0, 1.0, 4.0, 10.0),
        ],
        TipToTipConfig::default(),
    );
    let policy = TipToTipEmpirePolicy {
        minimum_gain_over_two_lane: 0.0,
        maximum_waste_ratio: 1.0,
        maximum_unmatched_ratio: 0.75,
    };

    let report = TipToTipEmpire::audit(&[half_unmatched], policy).unwrap();

    assert_eq!(report.unmatched_count, 2);
    assert_eq!(report.rooms[0].signals, [EmpireSignal::OnPlan]);
}

#[test]
fn empire_policy_reuses_numeric_validation() {
    assert_eq!(
        TipToTipEmpire::audit(
            &[room("garage", 10.0)],
            TipToTipEmpirePolicy {
                minimum_gain_over_two_lane: -1.0,
                ..TipToTipEmpirePolicy::default()
            },
        ),
        Err(TipToTipError::InvalidNumber {
            field: "minimum_gain_over_two_lane",
            value: -1.0,
        })
    );

    match TipToTipEmpire::audit(
        &[room("garage", 10.0)],
        TipToTipEmpirePolicy {
            maximum_waste_ratio: f64::NAN,
            ..TipToTipEmpirePolicy::default()
        },
    ) {
        Err(TipToTipError::InvalidNumber { field, value }) => {
            assert_eq!(field, "maximum_waste_ratio");
            assert!(value.is_nan());
        }
        result => panic!("expected NaN policy validation error, got {result:?}"),
    }
}
