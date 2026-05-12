use std::mem;

use crate::{enterprise::TipToTipRoom, profile::D2fProfile};

const GPU_TOTAL_PROFILE_THRESHOLD: usize = 1_000_000;
const GPU_ROOM_THRESHOLD: usize = 128;
const GPU_LARGEST_ROOM_THRESHOLD: usize = 4_096;

/// Backend recommendation for a tip-to-tip workload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeBackend {
    /// Use the deterministic in-process CPU planner.
    Cpu,
    /// A future GPU backend may be worth dispatching for this workload shape.
    GpuCandidate,
}

/// Workload shape used for acceleration decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadShape {
    /// No profiles.
    Empty,
    /// A normal room-sized input where CPU overhead is lower than GPU dispatch.
    RoomScale,
    /// A single large room where sorting still dominates the planner.
    LoadScaleRoom,
    /// Many rooms, but not enough total work to amortize GPU transfer overhead.
    Portfolio,
    /// Many large rooms that can amortize transfer and kernel launch overhead.
    GpuScalePortfolio,
}

/// Planner phase that could plausibly be offloaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuOffloadPhase {
    /// No GPU phase is recommended for this workload.
    None,
    /// Independent pair compatibility screening across many sorted rooms.
    PairScreening,
    /// Batched room-level metric reductions after compatibility screening.
    PortfolioReductions,
}

/// A hardware-neutral acceleration recommendation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipToTipAccelerationReport {
    /// Recommended backend class.
    pub backend: ComputeBackend,
    /// Classified workload shape.
    pub workload_shape: WorkloadShape,
    /// Phase that could be offloaded if a GPU backend is available.
    pub offload_phase: GpuOffloadPhase,
    /// Number of rooms included in the decision.
    pub room_count: usize,
    /// Total profiles included in the decision.
    pub total_profiles: usize,
    /// Largest room profile count.
    pub largest_room_profiles: usize,
    /// Approximate one-way payload size for profile data.
    pub estimated_profile_bytes: usize,
    /// Short rationale suitable for logs and CLI output.
    pub rationale: &'static str,
}

/// Acceleration advisor for CPU/GPU routing.
///
/// The current crate ships only the deterministic CPU implementation. This
/// advisor makes the GPU boundary explicit so future backends can be added
/// behind a stable decision policy.
#[derive(Debug, Clone, Copy, Default)]
pub struct TipToTipAcceleration;

impl TipToTipAcceleration {
    /// Assess a single room of profiles.
    pub fn assess_profiles(profiles: &[D2fProfile]) -> TipToTipAccelerationReport {
        Self::assess_counts(1, profiles.len(), profiles.len())
    }

    /// Assess an enterprise portfolio of rooms.
    pub fn assess_rooms(rooms: &[TipToTipRoom]) -> TipToTipAccelerationReport {
        let total_profiles = rooms.iter().map(|room| room.profiles.len()).sum();
        let largest_room_profiles = rooms
            .iter()
            .map(|room| room.profiles.len())
            .max()
            .unwrap_or(0);

        Self::assess_counts(rooms.len(), total_profiles, largest_room_profiles)
    }

    /// Assess a synthetic workload without allocating profiles.
    pub fn assess_counts(
        room_count: usize,
        total_profiles: usize,
        largest_room_profiles: usize,
    ) -> TipToTipAccelerationReport {
        let estimated_profile_bytes = total_profiles.saturating_mul(mem::size_of::<D2fProfile>());

        let (backend, workload_shape, offload_phase, rationale) = if total_profiles == 0 {
            (
                ComputeBackend::Cpu,
                WorkloadShape::Empty,
                GpuOffloadPhase::None,
                "empty workload; CPU return path is already constant-time",
            )
        } else if room_count >= GPU_ROOM_THRESHOLD
            && total_profiles >= GPU_TOTAL_PROFILE_THRESHOLD
            && largest_room_profiles >= GPU_LARGEST_ROOM_THRESHOLD
        {
            (
                ComputeBackend::GpuCandidate,
                WorkloadShape::GpuScalePortfolio,
                GpuOffloadPhase::PairScreening,
                "large batched portfolio can amortize transfer and kernel launch overhead",
            )
        } else if room_count > 1 {
            (
                ComputeBackend::Cpu,
                WorkloadShape::Portfolio,
                GpuOffloadPhase::None,
                "portfolio is not large enough to amortize GPU transfer overhead",
            )
        } else if total_profiles >= GPU_LARGEST_ROOM_THRESHOLD {
            (
                ComputeBackend::Cpu,
                WorkloadShape::LoadScaleRoom,
                GpuOffloadPhase::None,
                "single-room planner is sort-heavy and branchy; CPU remains the deterministic path",
            )
        } else {
            (
                ComputeBackend::Cpu,
                WorkloadShape::RoomScale,
                GpuOffloadPhase::None,
                "room-scale workload is below practical GPU dispatch thresholds",
            )
        };

        TipToTipAccelerationReport {
            backend,
            workload_shape,
            offload_phase,
            room_count,
            total_profiles,
            largest_room_profiles,
            estimated_profile_bytes,
            rationale,
        }
    }
}
