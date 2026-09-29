//! The vehicle as a page sees it: `window.omsi.vehicle`.
//!
//! OMSI buses name their script variables as they please (`door_0`, `Fahrertuer_Rechts`,
//! `elec_busbar_main` ...), so a page that wants "is the engine running?" would have to
//! know every bus. This module turns the variables into one **normalised snapshot** with
//! fixed names, using the same conventions the game itself uses (the engine check of the
//! start-up helper, the LAN pose, the door logic of the passengers). Anything a bus does not
//! have shows up as `null` (numbers) or `false` (switches), never as a missing property, so
//! a page can read `omsi.vehicle.engine.rpm` without guarding.
//!
//! Variables the snapshot does not cover are still reachable: `omsi.vars.num`,
//! `omsi.vars.str` and `omsi.getVar(name)` give every variable of the bus's own variable list.
//!
//! The snapshot is a tree of [`ApiValue`]s that any [`crate::htmltex::HtmlRenderer`] backend
//! can turn into its own objects. Numbers are rounded to a sensible step so that a value that
//! merely jitters in the last digit does not redraw the page every frame.
//!
//! # Layout (API version 1)
//!
//! | path | meaning |
//! | --- | --- |
//! | `api` | version of this layout (1) |
//! | `info.number`, `.ident`, `.yard`, `.route`, `.nextStop` | text variables of the bus |
//! | `motion.speedKmh`, `.heading`, `.pitch`, `.bank`, `.steeringDeg`, `.x`, `.y`, `.z`, `.odometerKm` | movement and place |
//! | `engine.running`, `.rpm`, `.throttle`, `.brake`, `.clutch`, `.gear`, `.tankContent` | drive train and pedals |
//! | `electrics.on`, `.busbarMain`, `.busbarAvailable`, `.failure` | on-board network |
//! | `battery.on` | battery switch (`null` when the bus has none) |
//! | `doors.count`, `.anyOpen`, `.list[i].number`, `.open`, `.isOpen` | door leaves, `list[0]` is door 1 |
//! | `passengers.onboard`, `.entries[i]`, `.exits[i]` (`number`, `open`, `requested`) | boarding state |
//! | `lights.headlights` (0-3), `.brake`, `.reverse`, `.fog`, `.indicator` (0-3), `.indicatorLeft`, `.indicatorRight`, `.hazard`, `.interior` | lamps |
//! | `brakes.parking`, `.stop`, `.kneeling` | brake and kneeling switches |
//! | `wipers.running` | windscreen wipers |
//! | `cabin.temperature` | cabin air, °C |
//! | `condition.dirt`, `.crashes`, `.lastImpactKJ`, `.streetCondition` | wear and road |
//! | `train.trailers` | coupled vehicles behind this one |

use crate::vehicle::VehicleInstance;

/// A value of the snapshot tree.
#[derive(Clone, Debug, PartialEq)]
pub enum ApiValue {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    List(Vec<ApiValue>),
    /// Properties in the order they are listed in.
    Map(Vec<(String, ApiValue)>),
}

impl ApiValue {
    /// A property of a [`ApiValue::Map`].
    pub fn get(&self, key: &str) -> Option<&ApiValue> {
        match self {
            ApiValue::Map(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// Everything the snapshot is made from. [`VehicleInstance::html_api_snapshot`] fills it from
/// a running vehicle; tests fill it by hand.
pub struct Inputs<'a> {
    /// A script variable by name (any letter case), `None` when the bus has none.
    pub var: &'a dyn Fn(&str) -> Option<f32>,
    /// A text variable by name (empty when the bus has none).
    pub text: &'a dyn Fn(&str) -> String,
    pub speed_kmh: f32,
    pub steer_deg: f32,
    pub heading: f64,
    pub pitch: f32,
    pub bank: f32,
    pub position: (f64, f64, f64),
    pub engine_running: bool,
    pub interior_light: f32,
    pub crashes: u32,
    pub last_impact_j: f32,
    pub dirt: f32,
    pub trailers: usize,
}

/// `x` rounded to `dp` decimals, as the number a page prints without float noise
/// (`12.3`, not `12.300000190734863`).
fn round_dp(x: f64, dp: i32) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    let p = 10f64.powi(dp);
    (x * p).round() / p
}

fn num(x: f32, dp: i32) -> ApiValue {
    ApiValue::Num(round_dp(x as f64, dp))
}

fn opt(x: Option<f32>, dp: i32) -> ApiValue {
    x.map_or(ApiValue::Null, |v| num(v, dp))
}

fn map(items: Vec<(&str, ApiValue)>) -> ApiValue {
    ApiValue::Map(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

/// Most doors of a bus the snapshot lists, and the most boarding doors (`PAX_Entry0..7`).
const MAX_DOORS: usize = 16;
const MAX_PAX_DOORS: usize = 8;

/// Build the snapshot.
pub fn snapshot(i: &Inputs) -> ApiValue {
    let var = i.var;
    // the first of some alternative names that the bus has
    let first = |names: &[&str]| names.iter().find_map(|n| var(n));
    let flag = |names: &[&str]| names.iter().any(|n| var(n).unwrap_or(0.0) > 0.5);
    // a switch that reads as `null` when the bus has none of the variables
    let opt_flag = |names: &[&str]| first(names).map(|v| v > 0.5);
    let on = |name: &str| var(name).unwrap_or(0.0) > 0.5;
    let flag_val = |b: bool| ApiValue::Bool(b);

    // ---- info
    let info = map(vec![
        ("number", ApiValue::Str((i.text)("number"))),
        ("ident", ApiValue::Str((i.text)("ident"))),
        ("yard", ApiValue::Str((i.text)("yard"))),
        ("route", ApiValue::Str((i.text)("act_route"))),
        ("nextStop", ApiValue::Str((i.text)("act_busstop"))),
    ]);

    // ---- motion
    let odometer = var("kmcounter_km").map(|km| km as f64 + var("kmcounter_m").unwrap_or(0.0) as f64 / 1000.0);
    let motion = map(vec![
        ("speedKmh", num(i.speed_kmh, 1)),
        ("heading", ApiValue::Num(round_dp(i.heading, 1))),
        ("pitch", num(i.pitch, 2)),
        ("bank", num(i.bank, 2)),
        ("steeringDeg", num(i.steer_deg, 1)),
        ("x", ApiValue::Num(round_dp(i.position.0, 2))),
        ("y", ApiValue::Num(round_dp(i.position.1, 2))),
        ("z", ApiValue::Num(round_dp(i.position.2, 2))),
        ("odometerKm", odometer.map_or(ApiValue::Null, |v| ApiValue::Num(round_dp(v, 3)))),
    ]);

    // ---- engine (`engine_n` is in rpm)
    let engine = map(vec![
        ("running", flag_val(i.engine_running)),
        ("rpm", opt(first(&["engine_n", "engine_rpm", "motor_n", "motor_rpm"]), 0)),
        ("throttle", opt(var("throttle"), 2)),
        ("brake", opt(var("brake"), 2)),
        ("clutch", opt(var("clutch"), 2)),
        ("gear", opt(first(&["antrieb_getr_aktugang", "gear"]), 0)),
        ("tankContent", opt(var("engine_tank_content"), 1)),
    ]);

    // ---- electrics and battery
    let busbar_main = on("elec_busbar_main");
    let busbar_avail = on("elec_busbar_avail");
    let electrics = map(vec![
        ("on", flag_val(busbar_main || busbar_avail)),
        ("busbarMain", flag_val(busbar_main)),
        ("busbarAvailable", flag_val(busbar_avail)),
        ("failure", flag_val(on("elec_failure_general"))),
    ]);
    let battery = map(vec![(
        "on",
        opt_flag(&["elec_battery_on", "battery_on", "batterie_on"]).map_or(ApiValue::Null, ApiValue::Bool),
    )]);

    // ---- doors: `door_0`, `door_1` ... as far as the bus has them (0 shut, 1 open)
    let door_list: Vec<ApiValue> = (0..MAX_DOORS)
        .map_while(|n| var(&format!("door_{n}")).map(|open| (n, open)))
        .map(|(n, open)| {
            map(vec![
                ("number", ApiValue::Num((n + 1) as f64)),
                ("open", num(open, 2)),
                ("isOpen", ApiValue::Bool(open > 0.05)),
            ])
        })
        .collect();
    let door_open = |n: usize| var(&format!("door_{n}")).unwrap_or(0.0) > 0.05;

    // ---- boarding: `PAX_Entry<n>_Open/_Req`, `PAX_Exit<n>_Open/_Req`
    let pax = |kind: &str| -> Vec<ApiValue> {
        (0..MAX_PAX_DOORS)
            .filter_map(|n| {
                let open = var(&format!("PAX_{kind}{n}_Open"));
                let req = var(&format!("PAX_{kind}{n}_Req"));
                if open.is_none() && req.is_none() {
                    return None;
                }
                Some(map(vec![
                    ("number", ApiValue::Num((n + 1) as f64)),
                    ("open", ApiValue::Bool(open.unwrap_or(0.0) > 0.5)),
                    ("requested", ApiValue::Bool(req.unwrap_or(0.0) > 0.5)),
                ]))
            })
            .collect()
    };
    let entries = pax("Entry");
    let exits = pax("Exit");
    let pax_open = entries.iter().chain(exits.iter()).any(|d| d.get("open") == Some(&ApiValue::Bool(true)));
    // what the passengers are told is open, where the bus has that; else the door leaves
    // (the same rule the game's own hints use)
    let any_open = if entries.is_empty() && exits.is_empty() {
        (0..door_list.len()).any(door_open)
    } else {
        pax_open
    };
    let doors = map(vec![
        ("count", ApiValue::Num(door_list.len() as f64)),
        ("anyOpen", ApiValue::Bool(any_open)),
        ("list", ApiValue::List(door_list)),
    ]);
    let passengers = map(vec![
        ("onboard", opt(var("humans_count"), 0)),
        ("entries", ApiValue::List(entries)),
        ("exits", ApiValue::List(exits)),
    ]);

    // ---- lights
    let headlights = if on("lights_fern") {
        3
    } else if on("lights_abbl") || on("lights_main") || var("Spot_Select").is_some_and(|s| s >= 0.0) {
        2
    } else if on("lights_stand") {
        1
    } else {
        0
    };
    let (lamp_l, lamp_r) = (on("lights_blinker_l"), on("lights_blinker_r"));
    // the indicator switch where the script has one (0 off, 1 left, 2 right, 3 hazard),
    // else the lamps
    let indicator = if on("lights_sw_warnblinker") {
        3
    } else {
        match var("lights_sw_blinker") {
            Some(s) if (0.5..2.5).contains(&s) => s.round() as i32,
            _ => match (lamp_l, lamp_r) {
                (true, true) => 3,
                (true, false) => 1,
                (false, true) => 2,
                _ => 0,
            },
        }
    };
    let lights = map(vec![
        ("headlights", ApiValue::Num(headlights as f64)),
        ("brake", flag_val(on("lights_brems"))),
        ("reverse", flag_val(on("lights_rueckfahr"))),
        ("fog", flag_val(on("lights_nebelschluss"))),
        ("indicator", ApiValue::Num(indicator as f64)),
        ("indicatorLeft", flag_val(lamp_l)),
        ("indicatorRight", flag_val(lamp_r)),
        ("hazard", flag_val(indicator == 3)),
        ("interior", num(i.interior_light, 2)),
    ]);

    // ---- brakes, wipers, cabin, condition, train
    let brakes = map(vec![
        ("parking", flag_val(flag(&["bremse_feststell", "parking_brake"]))),
        ("stop", flag_val(flag(&["bremse_halte", "bremse_halte_sw", "bus_stop_brake"]))),
        ("kneeling", flag_val(flag(&["bremse_kneeling", "vdv_kneel", "ecas_kneel", "kneeling"]))),
    ]);
    let wipers = map(vec![("running", flag_val(flag(&["wiperrunning", "wiper_running"])))]);
    let cabin = map(vec![("temperature", opt(var("Cabinair_Temp"), 1))]);
    let condition = map(vec![
        ("dirt", num(i.dirt, 2)),
        ("crashes", ApiValue::Num(i.crashes as f64)),
        ("lastImpactKJ", num(i.last_impact_j / 1000.0, 1)),
        ("streetCondition", opt(var("StreetCond"), 2)),
    ]);
    let train = map(vec![("trailers", ApiValue::Num(i.trailers as f64))]);

    map(vec![
        ("api", ApiValue::Num(1.0)),
        ("info", info),
        ("motion", motion),
        ("engine", engine),
        ("electrics", electrics),
        ("battery", battery),
        ("doors", doors),
        ("passengers", passengers),
        ("lights", lights),
        ("brakes", brakes),
        ("wipers", wipers),
        ("cabin", cabin),
        ("condition", condition),
        ("train", train),
    ])
}

impl VehicleInstance {
    /// The normalised state of this vehicle for its HTML textures (see the module docs).
    pub fn html_api_snapshot(&self) -> ApiValue {
        let var = |n: &str| self.var(n);
        let text = |n: &str| self.str_var(n);
        snapshot(&Inputs {
            var: &var,
            text: &text,
            speed_kmh: self.physics.velocity_kmh(),
            steer_deg: self.physics.steer_deg,
            heading: self.heading,
            pitch: self.pitch,
            bank: self.bank,
            position: (self.position.x, self.position.y, self.position.z),
            engine_running: crate::startup::engine_running(self),
            interior_light: self.interior_light(),
            crashes: self.crashes,
            last_impact_j: self.last_impact,
            dirt: self.dirt,
            trailers: self.trailers.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn snap(vars: &[(&str, f32)], texts: &[(&str, &str)]) -> ApiValue {
        let v: HashMap<String, f32> = vars.iter().map(|(k, x)| (k.to_ascii_lowercase(), *x)).collect();
        let t: HashMap<String, String> = texts.iter().map(|(k, x)| (k.to_string(), x.to_string())).collect();
        let var = move |n: &str| v.get(&n.to_ascii_lowercase()).copied();
        let text = move |n: &str| t.get(n).cloned().unwrap_or_default();
        snapshot(&Inputs {
            var: &var,
            text: &text,
            speed_kmh: 42.349_998,
            steer_deg: 0.0,
            heading: 90.0,
            pitch: 0.0,
            bank: 0.0,
            position: (1.0, 2.0, 3.0),
            engine_running: true,
            interior_light: 0.5,
            crashes: 0,
            last_impact_j: 0.0,
            dirt: 0.0,
            trailers: 0,
        })
    }

    fn at<'a>(v: &'a ApiValue, path: &str) -> &'a ApiValue {
        path.split('.').fold(v, |cur, key| match (cur, key.parse::<usize>()) {
            (ApiValue::List(l), Ok(n)) => &l[n],
            _ => cur.get(key).unwrap_or_else(|| panic!("no {key} in {path}")),
        })
    }

    #[test]
    fn numbers_are_rounded_without_float_noise() {
        let s = snap(&[], &[]);
        assert_eq!(at(&s, "motion.speedKmh"), &ApiValue::Num(42.3));
    }

    #[test]
    fn a_bus_without_a_signal_reports_null_or_false() {
        let s = snap(&[], &[]);
        assert_eq!(at(&s, "engine.rpm"), &ApiValue::Null);
        assert_eq!(at(&s, "battery.on"), &ApiValue::Null);
        assert_eq!(at(&s, "doors.count"), &ApiValue::Num(0.0));
        assert_eq!(at(&s, "brakes.parking"), &ApiValue::Bool(false));
        assert_eq!(at(&s, "engine.running"), &ApiValue::Bool(true));
    }

    #[test]
    fn doors_are_listed_as_far_as_the_bus_has_them() {
        let s = snap(&[("door_0", 1.0), ("door_1", 0.0), ("door_2", 0.4)], &[]);
        assert_eq!(at(&s, "doors.count"), &ApiValue::Num(3.0));
        assert_eq!(at(&s, "doors.list.0.number"), &ApiValue::Num(1.0));
        assert_eq!(at(&s, "doors.list.0.isOpen"), &ApiValue::Bool(true));
        assert_eq!(at(&s, "doors.list.1.isOpen"), &ApiValue::Bool(false));
        assert_eq!(at(&s, "doors.list.2.open"), &ApiValue::Num(0.4));
        assert_eq!(at(&s, "doors.anyOpen"), &ApiValue::Bool(true));
        // a gap ends the list (door_4 without door_3 is not a door of this bus)
        let g = snap(&[("door_0", 0.0), ("door_2", 1.0)], &[]);
        assert_eq!(at(&g, "doors.count"), &ApiValue::Num(1.0));
    }

    #[test]
    fn what_the_passengers_are_told_decides_whether_a_door_is_open() {
        // a mod bus whose `door_0` is something else, but whose boarding doors are shut
        let s = snap(&[("door_0", 1.0), ("PAX_Entry0_Open", 0.0), ("PAX_Exit0_Open", 0.0)], &[]);
        assert_eq!(at(&s, "doors.anyOpen"), &ApiValue::Bool(false));
        let o = snap(&[("PAX_Entry0_Open", 1.0), ("PAX_Entry0_Req", 1.0)], &[]);
        assert_eq!(at(&o, "doors.anyOpen"), &ApiValue::Bool(true));
        assert_eq!(at(&o, "passengers.entries.0.requested"), &ApiValue::Bool(true));
    }

    #[test]
    fn engine_battery_lights_and_texts() {
        let s = snap(
            &[
                ("engine_n", 812.4),
                ("elec_busbar_main", 1.0),
                ("batterie_on", 1.0),
                ("lights_abbl", 1.0),
                ("lights_blinker_l", 1.0),
                ("bremse_feststell", 1.0),
            ],
            &[("number", "4711"), ("act_busstop", "Hauptbahnhof")],
        );
        assert_eq!(at(&s, "engine.rpm"), &ApiValue::Num(812.0));
        assert_eq!(at(&s, "electrics.on"), &ApiValue::Bool(true));
        assert_eq!(at(&s, "battery.on"), &ApiValue::Bool(true));
        assert_eq!(at(&s, "lights.headlights"), &ApiValue::Num(2.0));
        assert_eq!(at(&s, "lights.indicator"), &ApiValue::Num(1.0));
        assert_eq!(at(&s, "brakes.parking"), &ApiValue::Bool(true));
        assert_eq!(at(&s, "info.number"), &ApiValue::Str("4711".into()));
        assert_eq!(at(&s, "info.nextStop"), &ApiValue::Str("Hauptbahnhof".into()));
    }

    #[test]
    fn the_indicator_switch_wins_over_the_lamps() {
        let s = snap(&[("lights_sw_blinker", 2.0), ("lights_blinker_l", 1.0)], &[]);
        assert_eq!(at(&s, "lights.indicator"), &ApiValue::Num(2.0));
        let h = snap(&[("lights_sw_warnblinker", 1.0)], &[]);
        assert_eq!(at(&h, "lights.hazard"), &ApiValue::Bool(true));
    }
}
