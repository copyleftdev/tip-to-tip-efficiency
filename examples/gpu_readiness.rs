use tip_to_tip::{TipToTipAcceleration, TipToTipRoom};

fn main() {
    let report = TipToTipAcceleration::assess_rooms(&[] as &[TipToTipRoom]);

    println!("backend: {:?}", report.backend);
    println!("shape: {:?}", report.workload_shape);
    println!("offload phase: {:?}", report.offload_phase);
    println!("rationale: {}", report.rationale);
}
