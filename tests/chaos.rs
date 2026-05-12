use tip_to_tip::{
    D2fProfile, EmpireSignal, TipToTip, TipToTipConfig, TipToTipEmpire, TipToTipEmpirePolicy,
    TipToTipRoom,
};

const EPSILON: f64 = 1.0e-9;

#[derive(Debug, Clone, Copy)]
struct ChaosRng {
    state: u64,
}

impl ChaosRng {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.state
    }

    fn usize(&mut self, range: std::ops::Range<usize>) -> usize {
        range.start + (self.next_u64() as usize % (range.end - range.start))
    }

    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }

    fn f64(&mut self, min: f64, max: f64) -> f64 {
        let unit = (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64);
        min + (max - min) * unit
    }
}

fn chaotic_profile(id: usize, rng: &mut ChaosRng) -> D2fProfile {
    let cluster = (id / 4) as f64;
    let compatible = rng.usize(0..10) < 7;
    let d2f = if compatible {
        cluster + rng.f64(0.0, 0.75)
    } else {
        cluster * 8.0 + rng.f64(4.0, 18.0)
    };
    let length = rng.f64(0.25, 4.0);
    let girth = if compatible {
        4.0 + rng.f64(0.0, 0.45)
    } else {
        rng.f64(0.0, 12.0)
    };
    let t2o = rng.f64(0.0, 180.0);

    D2fProfile::new(id, d2f, length, girth, t2o)
}

fn assert_plan_invariants(profiles: &[D2fProfile], config: TipToTipConfig) {
    let plan = TipToTip::plan(profiles, config).unwrap();
    let paired_count = plan.pairs.len() * 2;
    let cohort_pair_count = plan
        .cohorts
        .iter()
        .map(|cohort| cohort.pair_indices.len())
        .sum::<usize>();

    assert_eq!(plan.total_participants, profiles.len());
    assert_eq!(paired_count + plan.unmatched_ids.len(), profiles.len());
    assert_eq!(cohort_pair_count, plan.pairs.len());
    assert!(plan.total_seconds.is_finite());
    assert!(plan.total_seconds >= 0.0);
    assert!(plan.fallback_seconds >= 0.0);
    assert!(plan.tip_to_tip_seconds >= 0.0);
    assert!(plan.wasted_seconds >= 0.0);

    for pair in &plan.pairs {
        assert!(pair.d2f_gap <= pair.bridge + EPSILON);
        assert!(pair.girth_gap <= config.girth_tolerance + EPSILON);
        if !config.allow_hot_swap {
            assert!(pair.t2o_gap_seconds <= config.t2o_tolerance_seconds + EPSILON);
        }
    }

    let mut expected_ids = profiles.to_vec();
    expected_ids.sort_by(|left, right| {
        left.d2f
            .total_cmp(&right.d2f)
            .then_with(|| left.id.cmp(&right.id))
    });
    assert_eq!(
        plan.sorted_ids_by_d2f,
        expected_ids
            .iter()
            .map(|profile| profile.id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn deterministic_chaos_rooms_preserve_planner_invariants() {
    let mut rng = ChaosRng::new(0x5449_5053_324f_5449);

    for round in 0..128 {
        let count = rng.usize(0..257);
        let profiles = (0..count)
            .map(|id| chaotic_profile((round * 1_000) + id, &mut rng))
            .collect::<Vec<_>>();
        let config = TipToTipConfig {
            girth_tolerance: rng.f64(0.0, 2.5),
            t2o_tolerance_seconds: rng.f64(0.0, 30.0),
            allow_hot_swap: rng.bool(),
            fallback_lanes: rng.usize(1..9),
        };

        assert_plan_invariants(&profiles, config);
    }
}

#[test]
fn enterprise_chaos_rollups_match_room_totals() {
    let mut rng = ChaosRng::new(0x454d_5049_5245_5449);
    let mut rooms = Vec::new();

    for room_index in 0..32 {
        let count = rng.usize(0..96);
        let profiles = (0..count)
            .map(|id| chaotic_profile((room_index * 10_000) + id, &mut rng))
            .collect::<Vec<_>>();
        let config = TipToTipConfig {
            girth_tolerance: rng.f64(0.0, 3.0),
            t2o_tolerance_seconds: rng.f64(0.0, 45.0),
            allow_hot_swap: rng.bool(),
            fallback_lanes: rng.usize(1..8),
        };

        rooms.push(TipToTipRoom::new(
            format!("chaos-room-{room_index:02}"),
            profiles,
            config,
        ));
    }

    let report = TipToTipEmpire::audit(
        &rooms,
        TipToTipEmpirePolicy {
            minimum_gain_over_two_lane: 0.0,
            maximum_waste_ratio: f64::MAX,
            maximum_unmatched_ratio: 1.0,
        },
    )
    .unwrap();

    let total_seconds = report
        .rooms
        .iter()
        .map(|room| room.plan.total_seconds)
        .sum::<f64>();
    let wasted_seconds = report
        .rooms
        .iter()
        .map(|room| room.plan.wasted_seconds)
        .sum::<f64>();
    let unmatched_count = report
        .rooms
        .iter()
        .map(|room| room.plan.unmatched_ids.len())
        .sum::<usize>();

    assert_eq!(report.rooms.len(), rooms.len());
    assert!((report.total_seconds - total_seconds).abs() <= EPSILON);
    assert!((report.wasted_seconds - wasted_seconds).abs() <= EPSILON);
    assert_eq!(report.unmatched_count, unmatched_count);
    assert!(report
        .rooms
        .iter()
        .all(|room| room.signals == [EmpireSignal::OnPlan]
            || room.signals == [EmpireSignal::EmptyRoom]));
}

#[test]
fn invalid_numeric_storm_is_rejected_without_partial_plans() {
    let invalid_values = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -EPSILON];

    for value in invalid_values {
        let profile = D2fProfile::new(1, value, 1.0, 1.0, 1.0);
        assert!(TipToTip::plan(&[profile], TipToTipConfig::default()).is_err());

        let profile = D2fProfile::new(1, 1.0, value, 1.0, 1.0);
        assert!(TipToTip::plan(&[profile], TipToTipConfig::default()).is_err());

        let profile = D2fProfile::new(1, 1.0, 1.0, value, 1.0);
        assert!(TipToTip::plan(&[profile], TipToTipConfig::default()).is_err());

        let profile = D2fProfile::new(1, 1.0, 1.0, 1.0, value);
        assert!(TipToTip::plan(&[profile], TipToTipConfig::default()).is_err());
    }
}
