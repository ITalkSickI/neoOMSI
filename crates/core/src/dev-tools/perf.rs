#![allow(unused_imports)]
use super::types::*;
use imgui::Condition;
use std::collections::BTreeMap;

pub(super) struct PerfTool {
    prev: BTreeMap<&'static str, f64>,
    prev_frames: u32,
    pub smooth: BTreeMap<&'static str, f32>,
    pub peak: BTreeMap<&'static str, f32>,
    pub frozen: bool,
    pub nested: bool,
    pub graph: Vec<f32>,
}

impl PerfTool {
    pub(super) fn new() -> PerfTool {
        PerfTool {
            prev: BTreeMap::new(),
            prev_frames: 0,
            smooth: BTreeMap::new(),
            peak: BTreeMap::new(),
            frozen: false,
            nested: true,
            graph: Vec::new(),
        }
    }

    /// Take this frame's share of the cumulative timers (per frame, even if frames were skipped).
    pub(super) fn update(&mut self, extra: &Extra, dt_ms: f32) {
        let frames = extra.frames.wrapping_sub(self.prev_frames).max(1) as f64;
        let first = self.prev_frames == 0;
        for (k, v) in &extra.profile {
            let before = self.prev.get(k).copied().unwrap_or(*v);
            let ms = (((v - before) / frames) * 1000.0).max(0.0) as f32;
            if first || self.frozen {
                continue;
            }
            let s = self.smooth.entry(k).or_insert(ms);
            *s += (ms - *s) * 0.1;
            let p = self.peak.entry(k).or_insert(0.0);
            *p = (*p * 0.995).max(ms);
        }
        self.prev = extra.profile.iter().copied().collect();
        self.prev_frames = extra.frames;
        if !self.frozen {
            self.graph.push(dt_ms);
            if self.graph.len() > 240 {
                self.graph.remove(0);
            }
        }
    }
}

fn percentile(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() - 1) as f32 * p).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

pub(super) fn window(ui: &imgui::Ui, open: &mut bool, tool: &mut PerfTool, snap: &Snapshot, extra: &Extra) {
    if !*open {
        return;
    }
    ui.window("Performance")
        .opened(open)
        .size([460.0, 520.0], Condition::FirstUseEver)
        .position([12.0, 32.0], Condition::FirstUseEver)
        .build(|| {
            let mut sorted = tool.graph.clone();
            sorted.sort_by(|a, b| a.total_cmp(b));
            let avg = if sorted.is_empty() {
                0.0
            } else {
                sorted.iter().sum::<f32>() / sorted.len() as f32
            };
            let p99 = percentile(&sorted, 0.99);
            let max = sorted.last().copied().unwrap_or(0.0);
            ui.text(format!("{:.0} FPS, {:.2} ms", snap.fps, snap.dt_ms));
            ui.text(format!("avg {avg:.1} ms   p99 {p99:.1} ms   max {max:.1} ms"));
            if p99 > 0.0 {
                ui.text(format!("1% low: {:.0} FPS", 1000.0 / p99));
            }
            ui.plot_lines("##perfms", &tool.graph)
                .scale_min(0.0)
                .scale_max(max.max(1.0))
                .graph_size([0.0, 64.0])
                .overlay_text(format!("max {max:.1} ms"))
                .build();
            ui.checkbox("Freeze", &mut tool.frozen);
            ui.same_line();
            ui.checkbox("Sub-sections", &mut tool.nested);
            ui.same_line();
            if ui.button("Reset peaks") {
                tool.peak.clear();
            }
            ui.separator();
            let mut rows: Vec<(&'static str, f32, f32)> = tool
                .smooth
                .iter()
                .filter(|(k, _)| tool.nested || !k.contains('.'))
                .map(|(k, v)| (*k, *v, tool.peak.get(k).copied().unwrap_or(0.0)))
                .filter(|r| r.1 > 0.005)
                .collect();
            rows.sort_by(|a, b| b.1.total_cmp(&a.1));
            let top = rows.first().map(|r| r.1).unwrap_or(1.0).max(0.001);
            let stages: f32 = tool
                .smooth
                .iter()
                .filter(|(k, _)| !k.contains('.'))
                .map(|(_, v)| *v)
                .sum();
            ui.text(format!(
                "Stages {stages:.2} ms of {:.2} ms (rest: {:.2} ms outside the timers)",
                snap.dt_ms,
                (snap.dt_ms - stages).max(0.0)
            ));
            ui.columns(3, "##perfcols", true);
            ui.text("Section");
            ui.next_column();
            ui.text("avg ms");
            ui.next_column();
            ui.text("peak ms");
            ui.next_column();
            ui.separator();
            for (k, v, p) in &rows {
                let hot = *v > 4.0;
                let c = if hot {
                    [1.0, 0.45, 0.3, 1.0]
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                };
                ui.text_colored(c, k);
                ui.next_column();
                ui.text(format!("{v:.2}"));
                ui.same_line();
                ui.text_disabled(format!("{:.0}%", v / top * 100.0));
                ui.next_column();
                ui.text(format!("{p:.2}"));
                ui.next_column();
            }
            ui.columns(1, "##perfend", false);
            if let Some(t) = extra.traffic.as_ref() {
                ui.separator();
                ui.text(format!(
                    "Traffic: {} cars, {} asleep",
                    t.cars, t.dormant
                ));
            }
            ui.text(format!(
                "Instances {}, Meshes {}, Lights {}",
                snap.instances, snap.meshes, snap.lights
            ));
        });
}