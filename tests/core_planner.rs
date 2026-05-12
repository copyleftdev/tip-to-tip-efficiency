use std::f64::consts::FRAC_PI_2;

use tip_to_tip::{D2fProfile, TipToTip, TipToTipConfig, TipToTipError};

const EPSILON: f64 = 1.0e-9;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {actual} to be within {EPSILON} of {expected}"
    );
}

fn profile(id: usize, d2f: f64, t2o: f64) -> D2fProfile {
    D2fProfile::new(id, d2f, 2.0, 4.0, t2o)
}

#[test]
fn dinesh_closed_form_divides_total_work_by_lanes() {
    assert_eq!(TipToTip::dinesh_closed_form(800, 10.0, 2), Ok(4000.0));
    assert_eq!(TipToTip::ideal_middle_out_seconds(800, 10.0), Ok(2000.0));
}

#[test]
fn planner_sorts_by_d2f_then_id_before_pairing() {
    let profiles = [
        profile(4, 2.0, 10.0),
        profile(2, 1.0, 10.0),
        profile(1, 1.0, 10.0),
        profile(3, 3.0, 10.0),
    ];

    let plan = TipToTip::plan(&profiles, TipToTipConfig::default()).unwrap();

    assert_eq!(plan.sorted_ids_by_d2f, [1, 2, 4, 3]);
    assert_eq!(plan.pairs[0].ids, [1, 2]);
    assert_eq!(plan.pairs[1].ids, [4, 3]);
}

#[test]
fn theta_d_uses_the_bridge_ratio() {
    let profiles = [
        D2fProfile::new(1, 0.0, 2.0, 4.0, 10.0),
        D2fProfile::new(2, 3.0, 3.0, 4.0, 10.0),
    ];

    let plan = TipToTip::plan(&profiles, TipToTipConfig::default()).unwrap();

    assert_close(plan.pairs[0].d2f_gap, 3.0);
    assert_close(plan.pairs[0].bridge, 5.0);
    assert_close(plan.pairs[0].theta_d_radians, 0.6_f64.asin());
}

#[test]
fn impossible_bridge_profiles_fall_back_to_conventional_lanes() {
    let profiles = [
        D2fProfile::new(1, 0.0, 1.0, 4.0, 10.0),
        D2fProfile::new(2, 10.0, 1.0, 4.0, 14.0),
    ];

    let plan = TipToTip::plan(&profiles, TipToTipConfig::default()).unwrap();

    assert!(plan.pairs.is_empty());
    assert_eq!(plan.unmatched_ids, [1, 2]);
    assert_close(plan.fallback_seconds, 12.0);
    assert_close(plan.total_seconds, 12.0);
}

#[test]
fn hot_swap_removes_the_t2o_compatibility_constraint() {
    let profiles = [profile(1, 0.0, 10.0), profile(2, 1.0, 100.0)];
    let default_plan = TipToTip::plan(&profiles, TipToTipConfig::default()).unwrap();
    let hot_swap_plan = TipToTip::plan(
        &profiles,
        TipToTipConfig {
            allow_hot_swap: true,
            ..TipToTipConfig::default()
        },
    )
    .unwrap();

    assert!(default_plan.pairs.is_empty());
    assert_eq!(hot_swap_plan.pairs.len(), 1);
    assert_close(hot_swap_plan.pairs[0].duration_seconds, 55.0);
    assert_close(hot_swap_plan.cohorts[0].duration_seconds, 55.0);
    assert_close(hot_swap_plan.total_seconds, 55.0);
    assert_close(hot_swap_plan.wasted_seconds, 0.0);
}

#[test]
fn hot_swap_cohort_averages_all_active_participants() {
    let profiles = [
        profile(1, 0.0, 10.0),
        profile(2, 1.0, 20.0),
        profile(3, 2.0, 30.0),
        profile(4, 3.0, 40.0),
    ];
    let plan = TipToTip::plan(
        &profiles,
        TipToTipConfig {
            allow_hot_swap: true,
            ..TipToTipConfig::default()
        },
    )
    .unwrap();

    assert_eq!(plan.cohorts[0].participant_count, 4);
    assert_close(plan.cohorts[0].duration_seconds, 25.0);
    assert_close(plan.total_seconds, 25.0);
    assert_close(plan.two_lane_baseline_seconds, 50.0);
    assert_close(plan.throughput_gain_over_two_lane, 2.0);
}

#[test]
fn odd_profile_uses_fallback_lanes_after_pairs_are_built() {
    let profiles = [
        profile(1, 0.0, 10.0),
        profile(2, 1.0, 10.0),
        profile(3, 2.0, 20.0),
    ];

    let plan = TipToTip::plan(&profiles, TipToTipConfig::default()).unwrap();

    assert_eq!(plan.pairs.len(), 1);
    assert_eq!(plan.unmatched_ids, [3]);
    assert_close(plan.tip_to_tip_seconds, 10.0);
    assert_close(plan.fallback_seconds, 10.0);
    assert_close(plan.total_seconds, 20.0);
}

#[test]
fn exact_d2f_boundary_is_feasible_and_reaches_a_right_angle() {
    let profiles = [
        D2fProfile::new(1, 0.0, 1.0, 4.0, 10.0),
        D2fProfile::new(2, 2.0, 1.0, 4.0, 10.0),
    ];

    let plan = TipToTip::plan(&profiles, TipToTipConfig::default()).unwrap();

    assert_eq!(plan.pairs.len(), 1);
    assert_close(plan.pairs[0].theta_d_radians, FRAC_PI_2);
}

#[test]
fn exact_girth_and_t2o_boundaries_are_feasible() {
    let profiles = [
        D2fProfile::new(1, 0.0, 2.0, 4.0, 10.0),
        D2fProfile::new(2, 1.0, 2.0, 4.5, 11.0),
    ];

    let plan = TipToTip::plan(&profiles, TipToTipConfig::default()).unwrap();

    assert_eq!(plan.pairs.len(), 1);
    assert_close(plan.pairs[0].girth_gap, 0.5);
    assert_close(plan.pairs[0].t2o_gap_seconds, 1.0);
}

#[test]
fn compatibility_thresholds_are_epsilon_inclusive() {
    let profiles = [
        D2fProfile::new(1, 0.0, 1.0, 4.0, 10.0),
        D2fProfile::new(
            2,
            2.0 + (EPSILON / 2.0),
            1.0,
            4.5 + (EPSILON / 2.0),
            11.0 + (EPSILON / 2.0),
        ),
    ];

    let plan = TipToTip::plan(&profiles, TipToTipConfig::default()).unwrap();

    assert_eq!(plan.pairs.len(), 1);
    assert!(plan.unmatched_ids.is_empty());
}

#[test]
fn compatibility_thresholds_accept_exact_epsilon_margin() {
    let d2f_plan = TipToTip::plan(
        &[
            D2fProfile::new(1, 0.0, 1.0, 4.0, 10.0),
            D2fProfile::new(2, 2.0 + EPSILON, 1.0, 4.0, 10.0),
        ],
        TipToTipConfig::default(),
    )
    .unwrap();
    let girth_plan = TipToTip::plan(
        &[
            D2fProfile::new(1, 0.0, 2.0, 0.0, 10.0),
            D2fProfile::new(2, 1.0, 2.0, 0.5 + EPSILON, 10.0),
        ],
        TipToTipConfig::default(),
    )
    .unwrap();
    let t2o_plan = TipToTip::plan(
        &[
            D2fProfile::new(1, 0.0, 2.0, 4.0, 0.0),
            D2fProfile::new(2, 1.0, 2.0, 4.0, 1.0 + EPSILON),
        ],
        TipToTipConfig::default(),
    )
    .unwrap();

    assert_eq!(d2f_plan.pairs.len(), 1);
    assert_eq!(girth_plan.pairs.len(), 1);
    assert_eq!(t2o_plan.pairs.len(), 1);
}

#[test]
fn compatibility_thresholds_reject_values_beyond_epsilon_margin() {
    let d2f_plan = TipToTip::plan(
        &[
            D2fProfile::new(1, 0.0, 1.0, 4.0, 10.0),
            D2fProfile::new(2, 2.0 + (EPSILON * 2.0), 1.0, 4.0, 10.0),
        ],
        TipToTipConfig::default(),
    )
    .unwrap();
    let girth_plan = TipToTip::plan(
        &[
            D2fProfile::new(1, 0.0, 2.0, 0.0, 10.0),
            D2fProfile::new(2, 1.0, 2.0, 0.5 + (EPSILON * 2.0), 10.0),
        ],
        TipToTipConfig::default(),
    )
    .unwrap();
    let t2o_plan = TipToTip::plan(
        &[
            D2fProfile::new(1, 0.0, 2.0, 4.0, 0.0),
            D2fProfile::new(2, 1.0, 2.0, 4.0, 1.0 + (EPSILON * 2.0)),
        ],
        TipToTipConfig::default(),
    )
    .unwrap();

    assert!(d2f_plan.pairs.is_empty());
    assert!(girth_plan.pairs.is_empty());
    assert!(t2o_plan.pairs.is_empty());
}

#[test]
fn pair_and_cohort_waste_are_counted_separately() {
    let profiles = [
        profile(1, 0.0, 10.0),
        profile(2, 1.0, 12.0),
        profile(3, 2.0, 10.0),
        profile(4, 3.0, 10.0),
    ];
    let config = TipToTipConfig {
        t2o_tolerance_seconds: 5.0,
        ..TipToTipConfig::default()
    };

    let plan = TipToTip::plan(&profiles, config).unwrap();

    assert_close(plan.pairs[0].wasted_seconds, 2.0);
    assert_close(plan.pairs[1].wasted_seconds, 0.0);
    assert_close(plan.cohorts[0].wasted_seconds, 4.0);
    assert_close(plan.wasted_seconds, 6.0);
    assert_close(plan.total_seconds, 12.0);
}

#[test]
fn cohort_indices_track_pair_positions() {
    let profiles = [
        profile(1, 0.0, 10.0),
        profile(2, 1.0, 10.0),
        profile(3, 2.0, 10.0),
        profile(4, 3.0, 10.0),
        profile(5, 4.0, 10.0),
        profile(6, 5.0, 10.0),
    ];

    let plan = TipToTip::plan(&profiles, TipToTipConfig::default()).unwrap();

    assert_eq!(plan.cohorts.len(), 2);
    assert_eq!(plan.cohorts[0].pair_indices, [0, 1]);
    assert_eq!(plan.cohorts[0].participant_count, 4);
    assert_eq!(plan.cohorts[1].pair_indices, [2]);
    assert_eq!(plan.cohorts[1].participant_count, 2);
}

#[test]
fn invalid_numbers_and_lane_counts_are_rejected() {
    assert_eq!(
        TipToTip::dinesh_closed_form(1, 1.0, 0),
        Err(TipToTipError::InvalidLaneCount)
    );

    assert_eq!(
        TipToTip::plan(
            &[profile(1, 0.0, 1.0)],
            TipToTipConfig {
                fallback_lanes: 0,
                ..TipToTipConfig::default()
            },
        ),
        Err(TipToTipError::InvalidLaneCount)
    );

    assert_eq!(
        TipToTip::plan(
            &[D2fProfile::new(1, -1.0, 1.0, 1.0, 1.0)],
            TipToTipConfig::default()
        ),
        Err(TipToTipError::InvalidNumber {
            field: "d2f",
            value: -1.0,
        })
    );

    match TipToTip::plan(
        &[D2fProfile::new(1, f64::NAN, 1.0, 1.0, 1.0)],
        TipToTipConfig::default(),
    ) {
        Err(TipToTipError::InvalidNumber { field, value }) => {
            assert_eq!(field, "d2f");
            assert!(value.is_nan());
        }
        result => panic!("expected NaN validation error, got {result:?}"),
    }
}

#[test]
fn error_display_is_precise_enough_for_callers() {
    assert_eq!(
        TipToTipError::InvalidNumber {
            field: "t2o",
            value: -7.0,
        }
        .to_string(),
        "t2o must be finite and non-negative, got -7"
    );
    assert_eq!(
        TipToTipError::InvalidLaneCount.to_string(),
        "lane count must be greater than zero"
    );
}
