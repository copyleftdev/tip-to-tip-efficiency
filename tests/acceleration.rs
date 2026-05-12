use std::mem;

use tip_to_tip::{
    ComputeBackend, D2fProfile, GpuOffloadPhase, TipToTipAcceleration, TipToTipConfig,
    TipToTipRoom, WorkloadShape,
};

fn profile(id: usize) -> D2fProfile {
    D2fProfile::new(id, id as f64, 2.0, 4.0, 10.0)
}

#[test]
fn empty_workloads_stay_on_the_cpu_return_path() {
    let report = TipToTipAcceleration::assess_counts(0, 0, 0);

    assert_eq!(report.backend, ComputeBackend::Cpu);
    assert_eq!(report.workload_shape, WorkloadShape::Empty);
    assert_eq!(report.offload_phase, GpuOffloadPhase::None);
    assert_eq!(report.estimated_profile_bytes, 0);
}

#[test]
fn normal_rooms_are_below_gpu_dispatch_thresholds() {
    let profiles = (0..1_024).map(profile).collect::<Vec<_>>();

    let report = TipToTipAcceleration::assess_profiles(&profiles);

    assert_eq!(report.backend, ComputeBackend::Cpu);
    assert_eq!(report.workload_shape, WorkloadShape::RoomScale);
    assert_eq!(report.offload_phase, GpuOffloadPhase::None);
    assert_eq!(
        report.estimated_profile_bytes,
        profiles.len() * mem::size_of::<D2fProfile>()
    );
}

#[test]
fn giant_single_rooms_still_prefer_cpu_determinism() {
    let report = TipToTipAcceleration::assess_counts(1, 1_000_000, 1_000_000);

    assert_eq!(report.backend, ComputeBackend::Cpu);
    assert_eq!(report.workload_shape, WorkloadShape::LoadScaleRoom);
    assert_eq!(report.offload_phase, GpuOffloadPhase::None);
}

#[test]
fn huge_batched_portfolios_become_gpu_candidates() {
    let report = TipToTipAcceleration::assess_counts(128, 1_000_000, 4_096);

    assert_eq!(report.backend, ComputeBackend::GpuCandidate);
    assert_eq!(report.workload_shape, WorkloadShape::GpuScalePortfolio);
    assert_eq!(report.offload_phase, GpuOffloadPhase::PairScreening);
    assert_eq!(
        report.estimated_profile_bytes,
        1_000_000 * mem::size_of::<D2fProfile>()
    );
}

#[test]
fn room_assessment_uses_actual_portfolio_shape() {
    let rooms = [
        TipToTipRoom::new(
            "alpha",
            (0..3).map(profile).collect::<Vec<_>>(),
            TipToTipConfig::default(),
        ),
        TipToTipRoom::new(
            "bravo",
            (3..8).map(profile).collect::<Vec<_>>(),
            TipToTipConfig::default(),
        ),
    ];

    let report = TipToTipAcceleration::assess_rooms(&rooms);

    assert_eq!(report.backend, ComputeBackend::Cpu);
    assert_eq!(report.workload_shape, WorkloadShape::Portfolio);
    assert_eq!(report.room_count, 2);
    assert_eq!(report.total_profiles, 8);
    assert_eq!(report.largest_room_profiles, 5);
}
