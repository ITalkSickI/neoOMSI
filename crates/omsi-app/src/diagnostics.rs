//! Why a bus does not move, and its physics in the log.

/// Why the bus is not moving although the throttle is pressed: the things a driver checks
/// first (empty when it moves or nothing is pressed). The window's HUD shows them, an
/// offscreen run logs them.
pub(crate) fn standing_reasons(v: &omsi_sim::VehicleInstance) -> Vec<String> {
    let mut lines = Vec::new();
    let throttle = v.var("throttle").unwrap_or(0.0) > 0.05;
    let slow = v.physics.velocity_kmh().abs() < 3.0;
    if !(throttle && slow) {
        return lines;
    }
    if !omsi_sim::startup::engine_running(v) {
        lines.push("The engine is off  (E electrics, M starter; mod buses with an ignition key turn it with E: press again and hold. Shift+U does it all)".to_string());
    } else if v
        .var("antrieb_getr_gangwahl")
        .map(|g| (g - 1.0).abs() < 0.1)
        .or_else(|| v.var("cockpit_gangwahltaster").map(|g| (g - 1.0).abs() < 0.1))
        .unwrap_or(false)
    {
        // gear position 1 is N in the stock gearboxes and the mods built on them
        lines.push("The gearbox is in N: press D (Shift+D when W A S D drive), or click it. Buses like the Citaro take D only with the brake held".to_string());
    } else if v.var("antrieb_getr_gangwahl").is_none()
        && v.var("cockpit_gangwahltaster").is_none()
        && v.var("antrieb_getr_gang").map(|g| g.abs() < 0.1).unwrap_or(false)
    {
        // a manual gearbox (the stock F90 and T3, the Sprinters, PAZ and LAZ mods): gear 0 is
        // neutral, and the keyboard's `kw_s_1` … are the digit keys
        lines.push("Manual gearbox in neutral: 1-5 select a gear (R reverse, N neutral); the clutch works itself unless auto_clutch=0".to_string());
    }
    let parking = v.var("bremse_feststell").or_else(|| v.var("parking_brake")).unwrap_or(0.0) > 0.5;
    if parking {
        lines.push("Parking brake is on  (. releases it)".to_string());
    }
    // a bus that stood long enough to lose its air (the stock scripts start with 4-9 bar):
    // the spring brake is released by air (`bremse_p_Brzyl_FBA`, absolute; the stock
    // brake curve still holds below 6.5 bar), so it holds until the compressor has
    // filled the tanks, whatever the parking brake lever says
    if !parking && lines.is_empty() {
        let fba_val = v.var("bremse_p_Brzyl_FBA").or_else(|| v.var("spring_brake_pressure"));
        if let Some(fba) = fba_val.filter(|p| *p < 6.0e5) {
            let tanks: Vec<f32> = (1..=4)
                .filter_map(|i| v.var(&format!("bremse_p_Tank0{i}")).or_else(|| v.var(&format!("air_tank_{i}"))))
                .collect();
            let low = tanks.iter().copied().fold(f32::MAX, f32::min);
            let tank = if tanks.is_empty() {
                String::new()
            } else {
                format!("{:.1} bar in the tanks, ", low / 1e5)
            };
            lines.push(format!("Air pressure is low ({tank}spring brake {:.1} bar): the spring brake holds until the compressor has filled the tanks - keep the engine running", fba / 1e5));
        }
    }
    if v.var("bremse_halte_sw").unwrap_or(0.0) > 0.5
        || v.var("bremse_halte").unwrap_or(0.0) > 0.5
        || v.var("bus_stop_brake").unwrap_or(0.0) > 0.5
    {
        lines.push(
            "Stop brake / door release is on: the bus stands until it is switched off".to_string(),
        );
    }
    if (0..4).any(|i| v.var(&format!("door_{i}")).unwrap_or(0.0) > 0.05) {
        lines.push("Doors are open".to_string());
    }
    lines
}

/// `OMSI_DEBUG_PHYSICS`: one line with the player's pose, speed, wheel contacts and crashes.
pub(crate) fn log_physics(v: &omsi_sim::VehicleInstance, t: f32) {
    let wheels = v
        .rigid
        .as_ref()
        .map(|rb| {
            rb.wheels
                .iter()
                .map(|w| {
                    format!(
                        "{:+.3}@{:+.3}{}{}{}",
                        w.compression,
                        w.ground_z - v.position.z,
                        if w.on_ground { "" } else { "!" },
                        if w.step_force.abs() > 500.0 {
                            format!(" step {:+.1} kN", w.step_force / 1000.0)
                        } else {
                            String::new()
                        },
                        if w.walls.is_empty() {
                            String::new()
                        } else {
                            format!(" against {} face(s)", w.walls.len())
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    // the drift of the body: how far its travel departs from where it points (deg), and the
    // coupled parts' angles to it
    let slip = v
        .rigid
        .as_ref()
        .filter(|rb| rb.velocity.truncate().length() > 1.0)
        .map(|rb| {
            let d = (rb.velocity.x as f64).atan2(rb.velocity.y as f64).to_degrees() - v.heading;
            let d = (d + 540.0).rem_euclid(360.0) - 180.0;
            let d = if d.abs() > 90.0 { (d + 360.0).rem_euclid(360.0) - 180.0 } else { d };
            format!(" slip {d:+.2}")
        })
        .unwrap_or_default();
    let joints = (0..2)
        .filter_map(|i| v.var(&format!("articulation_{i}_alpha")).map(|a| format!(" joint{i} {:+.2}", a)))
        .collect::<String>();
    log::info!(
        "physics t={t:5.2} pos ({:.2}, {:.2}, {:.3}) hdg {:.1}{slip}{joints} pitch {:+.2} bank {:+.2} v {:+.2} km/h wheels [{wheels}] crashes {} last {:.1} kJ gear {:?} n {:?} M_Wheel {:?} brake {:?} pedal {:?} abs {:?} rpm {:?}",
        v.position.x,
        v.position.y,
        v.position.z,
        v.heading,
        v.pitch,
        v.bank,
        v.physics.velocity_kmh(),
        v.crashes,
        v.last_impact / 1000.0,
        v.var("antrieb_getr_aktugang").or(v.var("gear")),
        v.var("engine_n"),
        v.var("M_Wheel"),
        v.var("Axle_Brakeforce_1_L"),
        v.var("Brake"),
        v.var("bremse_ABS_eingriff"),
        v.var("Wheel_RotationSpeed_1_L"),
    );
}
