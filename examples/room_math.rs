use tip_to_tip::{
    D2fProfile, TipToTip, TipToTipConfig, TipToTipEmpire, TipToTipEmpirePolicy, TipToTipRoom,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profiles = [
        D2fProfile::new(1, 10.0, 2.0, 4.0, 10.0),
        D2fProfile::new(2, 11.0, 2.0, 4.0, 10.0),
        D2fProfile::new(3, 12.0, 2.0, 4.0, 10.0),
        D2fProfile::new(4, 13.0, 2.0, 4.0, 10.0),
    ];

    let plan = TipToTip::plan(&profiles, TipToTipConfig::default())?;

    println!("two-lane baseline: {:.2}s", plan.two_lane_baseline_seconds);
    println!("tip-to-tip total: {:.2}s", plan.total_seconds);
    println!(
        "throughput gain: {:.2}x",
        plan.throughput_gain_over_two_lane
    );

    let room = TipToTipRoom::new("hacker-hostel", profiles, TipToTipConfig::default());
    let report = TipToTipEmpire::audit(&[room], TipToTipEmpirePolicy::default())?;

    println!("promoted rooms: {}", report.promoted_rooms);

    Ok(())
}
