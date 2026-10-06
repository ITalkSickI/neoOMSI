//! All default settings for neo
use toml::Value;

pub enum Def {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(&'static str),
}

impl Def {
    pub fn to_value(&self) -> Value {
        match self {
            Def::Bool(v) => Value::Boolean(*v),
            Def::Int(v) => Value::Integer(*v),
            Def::Float(v) => Value::Float(*v),
            Def::Str(v) => Value::String((*v).to_string()),
        }
    }
}

/// (category, key, default)
pub const DEFAULTS: &[(&str, &str, Def)] = &[
    // [gameplay]
    ("gameplay", "drive-keys", Def::Str("simple")),
    ("gameplay", "boarding", Def::Str("auto")),
    ("gameplay", "pax_prefer_seats", Def::Bool(false)),
    ("gameplay", "exact_fare", Def::Bool(true)),
    ("gameplay", "driver", Def::Bool(true)),
    ("gameplay", "maintenance", Def::Int(0)),
    ("gameplay", "collision_vehicles", Def::Bool(true)),
    ("gameplay", "collision_objects", Def::Bool(true)),
    ("gameplay", "collision_pedestrians", Def::Bool(true)),
    ("gameplay", "hands_in_cab", Def::Bool(false)),
    ("gameplay", "time_speed", Def::Float(1.0)),
    ("gameplay", "time_sync", Def::Bool(false)),
    ("gameplay", "metar_sync", Def::Bool(false)),
    ("gameplay", "metar_station", Def::Str("")),
    ("gameplay", "auto_clutch", Def::Bool(true)),
    ("gameplay", "momentary_gears", Def::Bool(false)),
    ("gameplay", "auto_ibis", Def::Bool(false)),
    ("gameplay", "auto_shift", Def::Bool(false)),
    // [graphics]
    ("graphics", "fullscreen", Def::Bool(false)),
    // [audio]
    ("audio", "master-volume", Def::Float(1.0)),
    // [controller]
    ("controller", "deadzone", Def::Float(0.05)),
    ("controller", "ff_enabled", Def::Bool(true)),
    ("controller", "ff_invert", Def::Bool(false)),
    ("controller", "assign", Def::Str("")),
    // [camera]
    ("camera", "fov", Def::Float(0.0)),
    ("camera", "collision", Def::Bool(true)),
    ("camera", "smooth", Def::Bool(true)),
    ("camera", "head_movement", Def::Bool(true)),
    ("camera", "steer_look", Def::Bool(false)),
    ("camera", "steer_look_angle", Def::Float(30.0)),
    ("camera", "steer_look_response", Def::Float(0.25)),
    ("camera", "seat_x", Def::Float(0.0)),
    ("camera", "seat_y", Def::Float(0.0)),
    ("camera", "seat_z", Def::Float(0.0)),
    ("camera", "look_sens", Def::Float(1.0)),
    ("camera", "alt_view", Def::Bool(true)),
    ("camera", "free_look", Def::Bool(false)),
    ("camera", "crosshair", Def::Bool(true)),
    ("camera", "head_tracking", Def::Bool(false)),
    ("camera", "head_tracking_port", Def::Int(4242)),
    ("camera", "head_tracking_invert", Def::Str("")),
    // [vr]
    ("vr", "enabled", Def::Bool(false)),
    ("vr", "scale", Def::Float(0.65)),
    ("vr", "head-smoothing-ms", Def::Float(0.0)),
    ("vr", "mirror-rate", Def::Float(16.0)),
    ("vr", "desktop-mirror", Def::Bool(true)),
];