//! Input actions: the names from `Inputs/keyboard.cfg` and how they reach a vehicle.
//!
//! Vehicle actions are script triggers with the same name; on key release the trigger
//! `<name>_off` fires. A few actions are handled by the engine itself (throttle, brake,
//! clutch, steering, views).

/// Actions the engine handles instead of forwarding to the script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineAction {
    Throttle,
    ThrottleAmplify,
    Brake,
    Clutch,
    SteeringLeft,
    SteeringRight,
    SteeringNeutral,
}

pub fn engine_action(name: &str) -> Option<EngineAction> {
    Some(match name.to_ascii_lowercase().as_str() {
        "throttle" => EngineAction::Throttle,
        "throttle_amplify" => EngineAction::ThrottleAmplify,
        "brake" => EngineAction::Brake,
        "clutch" => EngineAction::Clutch,
        "steering_left" => EngineAction::SteeringLeft,
        "steering_right" => EngineAction::SteeringRight,
        "steering_neutral" => EngineAction::SteeringNeutral,
        _ => return None,
    })
}

/// Keyboard-driven analogue inputs, integrated per frame like the original (keys ramp the
/// pedal/steering position instead of setting it).
#[derive(Debug, Clone, Default)]
pub struct KeyboardAxes {
    pub throttle_key: bool,
    pub amplify_key: bool,
    pub brake_key: bool,
    pub clutch_key: bool,
    pub left_key: bool,
    pub right_key: bool,
    pub neutral_key: bool,
    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
    pub steering: f32,
    /// Road speed in km/h: the steering returns to centre by itself only while rolling.
    pub speed_kmh: f32,
    /// Rate of the steering wheel (fraction per second) while it swings back on its own.
    pub steer_vel: f32,
}

impl KeyboardAxes {
    pub fn set(&mut self, action: EngineAction, pressed: bool) {
        match action {
            EngineAction::Throttle => self.throttle_key = pressed,
            EngineAction::ThrottleAmplify => self.amplify_key = pressed,
            EngineAction::Brake => self.brake_key = pressed,
            EngineAction::Clutch => self.clutch_key = pressed,
            EngineAction::SteeringLeft => self.left_key = pressed,
            EngineAction::SteeringRight => self.right_key = pressed,
            EngineAction::SteeringNeutral => self.neutral_key = pressed,
        }
    }

    /// Let go of every key: the window losing focus (alt-tab, a click outside it, an OS
    /// dialog) never delivers the matching key-up, so without this a throttle or steering
    /// key held at that moment stayed "pressed" forever (and, with a modifier key stuck the
    /// same way, a later plain key press could be misread as held with that modifier).
    pub fn release_all(&mut self) {
        *self = KeyboardAxes {
            throttle: self.throttle,
            brake: self.brake,
            clutch: self.clutch,
            steering: self.steering,
            speed_kmh: self.speed_kmh,
            ..Default::default()
        };
    }

    pub fn update(&mut self, dt: f32) {
        let ramp = |v: &mut f32, up: bool, speed_up: f32, speed_down: f32| {
            if up {
                *v = (*v + speed_up * dt).min(1.0);
            } else {
                *v = (*v - speed_down * dt).max(0.0);
            }
        };
        let amp = if self.amplify_key { 3.0 } else { 1.0 };
        ramp(&mut self.throttle, self.throttle_key, 1.5 * amp, 3.0);
        ramp(&mut self.brake, self.brake_key, 1.5, 3.0);
        ramp(&mut self.clutch, self.clutch_key, 3.0, 3.0);
        // Steering. A bus's wheel is about two and a half turns from lock to lock. It still
        // came back too slowly for how fast a key could turn it (1.25 s to full lock against
        // 15+ s to come back on its own), so every correction overshot and had to be walked
        // back by hand. Now the return (the castor of the front axle pulling the wheel to the
        // middle, harder the faster the bus rolls) is a little brisker standing still, and the
        // key turns the wheel at that same pace, only a tenth faster — never a swerve, because
        // a correction can only be as fast as the wheel would come back on its own anyway.
        let v = self.speed_kmh.abs();
        let base = 0.8 / (1.0 + v / 45.0);
        let back = base * (0.25 + 0.75 * (v / 25.0).min(1.0));
        let rate = back * 1.1;
        if self.neutral_key {
            self.steering = 0.0;
            self.steer_vel = 0.0;
        } else if self.left_key {
            self.steering = (self.steering - rate * dt).max(-1.0);
            self.steer_vel = 0.0;
        } else if self.right_key {
            self.steering = (self.steering + rate * dt).min(1.0);
            self.steer_vel = 0.0;
        } else {
            // Released: the wheel comes back at `back`, easing out over the last bit so that
            // it settles instead of stopping dead in the middle.
            let ease = (self.steering.abs() / 0.08).clamp(0.3, 1.0);
            let step = back * ease * dt;
            self.steering -= self.steering.clamp(-step, step);
            self.steer_vel = 0.0;
            // (a snap from farther out was a visible jolt of the wheel and the driver's hands)
            if self.steering.abs() < 0.0003 {
                self.steering = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A key turns the wheel at the pace it comes back to the middle with, only a tenth
    /// faster: a few seconds from the middle to full lock standing, quicker while rolling.
    /// Let go, it comes back at very nearly the speed it was turned with and settles in the
    /// middle without swinging through it; standing still it still comes back, just slowly.
    #[test]
    fn steering_returns_like_a_spring() {
        let step = 1.0 / 60.0;
        // standing: full lock takes a few seconds, not the old 1.25 s
        let mut s = KeyboardAxes::default();
        s.set(EngineAction::SteeringRight, true);
        let mut t_lock = None;
        for i in 1..=300 {
            s.update(step);
            if s.steering >= 1.0 && t_lock.is_none() {
                t_lock = Some(i as f32 * step);
            }
        }
        let t_lock = t_lock.expect("never reached full lock");
        assert!(
            (2.5..=5.0).contains(&t_lock),
            "middle to full lock standing took {t_lock} s"
        );
        // at 30 km/h a second of the key is under half a lock, and slower still at 80
        let mut k = KeyboardAxes {
            speed_kmh: 30.0,
            ..Default::default()
        };
        k.set(EngineAction::SteeringLeft, true);
        for _ in 0..60 {
            k.update(step);
        }
        assert!(
            k.steering < -0.3 && k.steering > -0.6,
            "held left for a second at 30 km/h: {}",
            k.steering
        );
        let mut fast = KeyboardAxes {
            speed_kmh: 80.0,
            ..Default::default()
        };
        fast.set(EngineAction::SteeringLeft, true);
        for _ in 0..60 {
            fast.update(step);
        }
        assert!(
            fast.steering > k.steering,
            "turned faster at 80 km/h ({}) than at 30 ({})",
            fast.steering,
            k.steering
        );
        // let go at 30 km/h: back in about the time it took (the key's pace, a tenth slower
        // than the turn), never through the middle
        k.set(EngineAction::SteeringLeft, false);
        let from = k.steering;
        let mut t_back = None;
        for i in 1..=300 {
            let before = k.steering;
            k.update(step);
            assert!(
                k.steering <= 0.0,
                "swung through the middle: {}",
                k.steering
            );
            assert!(
                k.steering >= before,
                "moved away from the middle: {} -> {}",
                before,
                k.steering
            );
            if k.steering == 0.0 && t_back.is_none() {
                t_back = Some(i as f32 * step);
            }
        }
        let t_back = t_back.expect("never settled");
        assert!(
            (0.8..=1.8).contains(&t_back),
            "back in {t_back} s from {from} (turned for 1 s)"
        );
        // standing still it still comes back, but slowly: well off centre after a second,
        // not stuck
        let mut s = KeyboardAxes {
            steering: -0.8,
            speed_kmh: 0.0,
            ..Default::default()
        };
        for _ in 0..60 {
            s.update(step);
        }
        assert!(
            s.steering < -0.3,
            "returned too fast standing still: {}",
            s.steering
        );
        assert!(
            s.steering > -0.78,
            "hardly came back standing still: {}",
            s.steering
        );
    }
}
