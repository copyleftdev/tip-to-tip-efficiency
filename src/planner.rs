use crate::{
    config::TipToTipConfig,
    constants::EPSILON,
    error::TipToTipError,
    plan::{TipToTipCohort, TipToTipPair, TipToTipPlan},
    profile::D2fProfile,
    validation::{validate_config, validate_non_negative, validate_profile},
};

pub(crate) fn dinesh_closed_form(
    participants: usize,
    mean_time_seconds: f64,
    simultaneous_lanes: usize,
) -> Result<f64, TipToTipError> {
    validate_non_negative("mean_time_seconds", mean_time_seconds)?;
    if simultaneous_lanes == 0 {
        return Err(TipToTipError::InvalidLaneCount);
    }

    Ok(participants as f64 * mean_time_seconds / simultaneous_lanes as f64)
}

pub(crate) fn ideal_middle_out_seconds(
    participants: usize,
    mean_time_seconds: f64,
) -> Result<f64, TipToTipError> {
    dinesh_closed_form(participants, mean_time_seconds, 4)
}

pub(crate) fn plan(
    profiles: &[D2fProfile],
    config: TipToTipConfig,
) -> Result<TipToTipPlan, TipToTipError> {
    validate_config(config)?;
    let mut total_work = 0.0;
    for profile in profiles {
        validate_profile(*profile)?;
        total_work += profile.t2o;
    }

    let mut sorted = profiles.to_vec();
    sorted.sort_by(|left, right| {
        left.d2f
            .total_cmp(&right.d2f)
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut pairs = Vec::with_capacity(sorted.len() / 2);
    let mut unmatched = Vec::with_capacity(sorted.len());
    let mut paired_waste = 0.0;
    let mut fallback_work_seconds = 0.0;

    for chunk in sorted.chunks(2) {
        match chunk {
            [left, right] => match build_pair(*left, *right, config) {
                Some(pair) => {
                    paired_waste += pair.wasted_seconds;
                    pairs.push(pair);
                }
                None => {
                    unmatched.push(left.id);
                    unmatched.push(right.id);
                    fallback_work_seconds += left.t2o + right.t2o;
                }
            },
            [single] => {
                unmatched.push(single.id);
                fallback_work_seconds += single.t2o;
            }
            _ => {}
        }
    }

    let (cohorts, tip_to_tip_seconds, cohort_waste) = build_cohorts(&pairs, config.allow_hot_swap);
    let fallback_seconds = fallback_work_seconds / config.fallback_lanes as f64;
    let total_seconds = tip_to_tip_seconds + fallback_seconds;
    let mean_t2o = if profiles.is_empty() {
        0.0
    } else {
        total_work / profiles.len() as f64
    };
    let two_lane_baseline_seconds = dinesh_closed_form(profiles.len(), mean_t2o, 2)?;
    let ideal_four_lane_seconds = ideal_middle_out_seconds(profiles.len(), mean_t2o)?;
    let throughput_gain_over_two_lane = if total_seconds <= EPSILON {
        1.0
    } else {
        two_lane_baseline_seconds / total_seconds
    };

    Ok(TipToTipPlan {
        total_participants: profiles.len(),
        sorted_ids_by_d2f: sorted.iter().map(|profile| profile.id).collect(),
        pairs,
        cohorts,
        unmatched_ids: unmatched,
        tip_to_tip_seconds,
        fallback_seconds,
        total_seconds,
        two_lane_baseline_seconds,
        ideal_four_lane_seconds,
        throughput_gain_over_two_lane,
        wasted_seconds: paired_waste + cohort_waste,
    })
}

fn build_pair(left: D2fProfile, right: D2fProfile, config: TipToTipConfig) -> Option<TipToTipPair> {
    let d2f_gap = (left.d2f - right.d2f).abs();
    let bridge = left.length + right.length;

    if d2f_gap > bridge + EPSILON {
        return None;
    }

    let girth_gap = (left.girth - right.girth).abs();
    if girth_gap > config.girth_tolerance + EPSILON {
        return None;
    }

    let t2o_gap_seconds = (left.t2o - right.t2o).abs();
    if !config.allow_hot_swap && t2o_gap_seconds > config.t2o_tolerance_seconds + EPSILON {
        return None;
    }

    let work_seconds = left.t2o + right.t2o;
    let duration_seconds = if config.allow_hot_swap {
        work_seconds / 2.0
    } else {
        left.t2o.max(right.t2o)
    };
    let wasted_seconds = if config.allow_hot_swap {
        0.0
    } else {
        (duration_seconds * 2.0) - work_seconds
    };
    let theta_d_radians = if bridge <= EPSILON {
        0.0
    } else {
        (d2f_gap / bridge).clamp(0.0, 1.0).asin()
    };

    Some(TipToTipPair {
        ids: [left.id, right.id],
        d2f_gap,
        bridge,
        theta_d_radians,
        girth_gap,
        t2o_gap_seconds,
        work_seconds,
        duration_seconds,
        wasted_seconds,
    })
}

fn build_cohorts(pairs: &[TipToTipPair], allow_hot_swap: bool) -> (Vec<TipToTipCohort>, f64, f64) {
    let mut cohorts = Vec::with_capacity(pairs.len().div_ceil(2));
    let mut tip_to_tip_seconds = 0.0;
    let mut cohort_waste = 0.0;

    for (chunk_index, chunk) in pairs.chunks(2).enumerate() {
        let first_pair_index = chunk_index * 2;
        let mut pair_indices = Vec::with_capacity(chunk.len());
        pair_indices.push(first_pair_index);
        if chunk.len() == 2 {
            pair_indices.push(first_pair_index + 1);
        }

        let participant_count = chunk.len() * 2;
        let (duration_seconds, wasted_seconds) = if allow_hot_swap {
            let work_seconds = chunk.iter().map(|pair| pair.work_seconds).sum::<f64>();
            (work_seconds / participant_count as f64, 0.0)
        } else {
            let first_duration = chunk[0].duration_seconds;
            let duration_seconds = if chunk.len() == 2 {
                first_duration.max(chunk[1].duration_seconds)
            } else {
                first_duration
            };
            let wasted_seconds = chunk
                .iter()
                .map(|pair| (duration_seconds - pair.duration_seconds) * 2.0)
                .sum();
            (duration_seconds, wasted_seconds)
        };

        tip_to_tip_seconds += duration_seconds;
        cohort_waste += wasted_seconds;
        cohorts.push(TipToTipCohort {
            pair_indices,
            participant_count,
            duration_seconds,
            wasted_seconds,
        });
    }

    (cohorts, tip_to_tip_seconds, cohort_waste)
}
