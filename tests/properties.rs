use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use tip_to_tip::{
    D2fProfile, TipToTip, TipToTipConfig, TipToTipEmpire, TipToTipEmpirePolicy, TipToTipRoom,
};

fn profile_strategy() -> impl Strategy<Value = D2fProfile> {
    (
        0usize..10_000,
        0.0f64..100.0,
        0.1f64..10.0,
        0.1f64..10.0,
        0.0f64..120.0,
    )
        .prop_map(|(id, d2f, length, girth, t2o)| D2fProfile::new(id, d2f, length, girth, t2o))
}

proptest! {
    #[test]
    fn dinesh_closed_form_is_total_work_divided_by_lanes(
        participants in 0usize..10_000,
        mean_time in 0.0f64..600.0,
        lanes in 1usize..16,
    ) {
        let seconds = TipToTip::dinesh_closed_form(participants, mean_time, lanes)
            .map_err(|err| TestCaseError::fail(err.to_string()))?;

        prop_assert!((seconds - (participants as f64 * mean_time / lanes as f64)).abs() < 1.0e-9);
    }

    #[test]
    fn perfect_four_lane_room_hits_the_middle_out_ideal(count in 0usize..256, t2o in 0.0f64..120.0) {
        let count = count - (count % 4);
        let profiles = (0..count)
            .map(|id| D2fProfile::new(id, id as f64, 2.0, 4.0, t2o))
            .collect::<Vec<_>>();
        let plan = TipToTip::plan(&profiles, TipToTipConfig::default())
            .map_err(|err| TestCaseError::fail(err.to_string()))?;

        prop_assert!(plan.unmatched_ids.is_empty());
        prop_assert!((plan.total_seconds - plan.ideal_four_lane_seconds).abs() < 1.0e-9);
        prop_assert!(plan.wasted_seconds.abs() < 1.0e-9);
    }

    #[test]
    fn plans_never_lose_participants(profiles in prop::collection::vec(profile_strategy(), 0..128)) {
        let plan = TipToTip::plan(&profiles, TipToTipConfig::default())
            .map_err(|err| TestCaseError::fail(err.to_string()))?;
        let paired_count = plan.pairs.len() * 2;
        let unmatched_count = plan.unmatched_ids.len();

        prop_assert_eq!(plan.total_participants, profiles.len());
        prop_assert_eq!(paired_count + unmatched_count, profiles.len());
    }

    #[test]
    fn sorted_ids_follow_d2f_then_id(profiles in prop::collection::vec(profile_strategy(), 0..128)) {
        let plan = TipToTip::plan(&profiles, TipToTipConfig::default())
            .map_err(|err| TestCaseError::fail(err.to_string()))?;
        let mut expected = profiles.clone();
        expected.sort_by(|left, right| {
            left.d2f
                .total_cmp(&right.d2f)
                .then_with(|| left.id.cmp(&right.id))
        });

        prop_assert_eq!(
            plan.sorted_ids_by_d2f,
            expected.iter().map(|profile| profile.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn hot_swap_has_no_idle_waste(profiles in prop::collection::vec(profile_strategy(), 0..128)) {
        let config = TipToTipConfig {
            allow_hot_swap: true,
            girth_tolerance: 10.0,
            ..TipToTipConfig::default()
        };
        let plan = TipToTip::plan(&profiles, config)
            .map_err(|err| TestCaseError::fail(err.to_string()))?;

        prop_assert!(plan.pairs.iter().all(|pair| pair.wasted_seconds == 0.0));
        prop_assert!(plan.cohorts.iter().all(|cohort| cohort.wasted_seconds == 0.0));
        prop_assert!(plan.wasted_seconds == 0.0);
    }

    #[test]
    fn empire_rollup_matches_sum_of_rooms(profiles in prop::collection::vec(profile_strategy(), 0..128), split in 0usize..128) {
        let split = split.min(profiles.len());
        let left = profiles[..split].to_vec();
        let right = profiles[split..].to_vec();
        let rooms = [
            TipToTipRoom::new("left", left.clone(), TipToTipConfig::default()),
            TipToTipRoom::new("right", right.clone(), TipToTipConfig::default()),
        ];
        let left_plan = TipToTip::plan(&left, TipToTipConfig::default())
            .map_err(|err| TestCaseError::fail(err.to_string()))?;
        let right_plan = TipToTip::plan(&right, TipToTipConfig::default())
            .map_err(|err| TestCaseError::fail(err.to_string()))?;
        let report = TipToTipEmpire::audit(
            &rooms,
            TipToTipEmpirePolicy {
                minimum_gain_over_two_lane: 0.0,
                maximum_waste_ratio: 1.0e9,
                maximum_unmatched_ratio: 1.0,
            },
        )
        .map_err(|err| TestCaseError::fail(err.to_string()))?;

        prop_assert_eq!(report.total_participants, profiles.len());
        prop_assert_eq!(report.unmatched_count, left_plan.unmatched_ids.len() + right_plan.unmatched_ids.len());
        prop_assert!((report.total_seconds - (left_plan.total_seconds + right_plan.total_seconds)).abs() < 1.0e-9);
        prop_assert!((report.two_lane_baseline_seconds - (left_plan.two_lane_baseline_seconds + right_plan.two_lane_baseline_seconds)).abs() < 1.0e-9);
        prop_assert!((report.wasted_seconds - (left_plan.wasted_seconds + right_plan.wasted_seconds)).abs() < 1.0e-9);
    }
}
