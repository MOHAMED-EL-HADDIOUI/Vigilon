use vigilon_core::metrics::BatteryInfo;

pub fn read() -> Option<BatteryInfo> {
    let manager = starship_battery::Manager::new().ok()?;
    let battery = manager.batteries().ok()?.flatten().next()?;
    let percentage = battery
        .state_of_charge()
        .get::<starship_battery::units::ratio::percent>();
    let health = battery
        .state_of_health()
        .get::<starship_battery::units::ratio::percent>();
    let time_to_empty = battery
        .time_to_empty()
        .map(|t| t.get::<starship_battery::units::time::second>() as u32);
    Some(BatteryInfo {
        present: true,
        state: format!("{:?}", battery.state()),
        percentage: Some(percentage),
        health_percent: Some(health),
        time_to_empty_sec: time_to_empty,
    })
}
