use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tip_to_tip::{
    D2fProfile, TipToTip, TipToTipConfig, TipToTipEmpire, TipToTipEmpirePolicy, TipToTipRoom,
};

fn benchmark_profiles(count: usize) -> Vec<D2fProfile> {
    (0..count)
        .map(|id| {
            let lane = id % 4;
            let cohort = id / 4;
            let d2f = cohort as f64 + lane as f64 * 0.20;
            let length = 1.25 + lane as f64 * 0.05;
            let girth = 4.0 + lane as f64 * 0.02;
            let t2o = 8.0 + (id % 11) as f64 * 0.25;

            D2fProfile::new(id, d2f, length, girth, t2o)
        })
        .collect()
}

fn fallback_storm_profiles(count: usize) -> Vec<D2fProfile> {
    (0..count)
        .map(|id| D2fProfile::new(id, id as f64 * 10.0, 0.10, 4.0, 8.0 + (id % 17) as f64))
        .collect()
}

fn benchmark_rooms(room_count: usize, profiles_per_room: usize) -> Vec<TipToTipRoom> {
    (0..room_count)
        .map(|room_index| {
            let mut profiles = benchmark_profiles(profiles_per_room);
            for profile in &mut profiles {
                profile.id += room_index * profiles_per_room;
                profile.d2f += room_index as f64 * 0.01;
            }

            TipToTipRoom::new(
                format!("room-{room_index:03}"),
                profiles,
                TipToTipConfig {
                    allow_hot_swap: room_index % 2 == 0,
                    girth_tolerance: 0.75,
                    t2o_tolerance_seconds: 3.0,
                    fallback_lanes: 2,
                },
            )
        })
        .collect()
}

fn bench_room_planner(c: &mut Criterion) {
    let mut group = c.benchmark_group("tip_to_tip_plan");

    for count in [16usize, 64, 256, 1024] {
        let profiles = benchmark_profiles(count);
        let config = TipToTipConfig {
            allow_hot_swap: true,
            girth_tolerance: 0.75,
            t2o_tolerance_seconds: 3.0,
            fallback_lanes: 2,
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &profiles,
            |b, profiles| {
                b.iter(|| TipToTip::plan(black_box(profiles), black_box(config)).unwrap())
            },
        );
    }

    group.finish();
}

fn bench_fallback_storm(c: &mut Criterion) {
    let mut group = c.benchmark_group("tip_to_tip_fallback_storm");

    for count in [1024usize, 4096, 16_384] {
        let profiles = fallback_storm_profiles(count);
        let config = TipToTipConfig::default();

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &profiles,
            |b, profiles| {
                b.iter(|| TipToTip::plan(black_box(profiles), black_box(config)).unwrap())
            },
        );
    }

    group.finish();
}

fn bench_enterprise_audit(c: &mut Criterion) {
    let mut group = c.benchmark_group("tip_to_tip_empire");

    for (room_count, profiles_per_room) in [(4usize, 64usize), (16, 64), (32, 128)] {
        let rooms = benchmark_rooms(room_count, profiles_per_room);
        let policy = TipToTipEmpirePolicy {
            minimum_gain_over_two_lane: 0.0,
            maximum_waste_ratio: 1.0,
            maximum_unmatched_ratio: 1.0,
        };

        group.bench_with_input(
            BenchmarkId::new("audit", format!("{room_count}x{profiles_per_room}")),
            &rooms,
            |b, rooms| {
                b.iter(|| TipToTipEmpire::audit(black_box(rooms), black_box(policy)).unwrap())
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_room_planner,
    bench_fallback_storm,
    bench_enterprise_audit
);
criterion_main!(benches);
