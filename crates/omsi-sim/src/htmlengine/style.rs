//! Computed style: lengths, colours and the property table a page can set.

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Len {
    Px(f32),
    Pct(f32),
}

impl Len {
    pub(crate) fn px(self, base: f32) -> f32 {
        match self {
            Len::Px(v) => v,
            Len::Pct(p) => base * p / 100.0,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Style {
    pub(crate) color: [u8; 4],
    pub(crate) bg: [u8; 4],
    pub(crate) font_px: f32,
    pub(crate) bold: bool,
    pub(crate) align: u8,
    pub(crate) line_h: f32,
    pub(crate) hidden: bool,
    pub(crate) margin: [f32; 4],
    pub(crate) margin_auto: bool,
    pub(crate) padding: [f32; 4],
    pub(crate) width: Option<Len>,
    pub(crate) height: Option<Len>,
    pub(crate) none: bool,
    pub(crate) inline: bool,
    /// `display:inline-block`: a box that sits in a row with its neighbours.
    pub(crate) inline_block: bool,
    pub(crate) radius: f32,
    /// The element this style was computed for (what a text run belongs to when it is hit).
    pub(crate) node: usize,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            color: [0, 0, 0, 255],
            bg: [0, 0, 0, 0],
            font_px: 16.0,
            bold: false,
            align: 0,
            line_h: 1.2,
            hidden: false,
            margin: [0.0; 4],
            margin_auto: false,
            padding: [0.0; 4],
            width: None,
            height: None,
            none: false,
            inline: false,
            inline_block: false,
            radius: 0.0,
            node: 0,
        }
    }
}

impl Style {
    pub(crate) fn inherit(&self) -> Style {
        Style {
            color: self.color,
            font_px: self.font_px,
            bold: self.bold,
            align: self.align,
            line_h: self.line_h,
            hidden: self.hidden,
            ..Style::default()
        }
    }
}

pub(crate) fn parse_color(s: &str) -> Option<[u8; 4]> {
    let s = s.trim().to_ascii_lowercase();
    if let Some(h) = s.strip_prefix('#') {
        let d: Vec<u8> = h.chars().map(|c| c.to_digit(16).map(|v| v as u8)).collect::<Option<Vec<_>>>()?;
        return match d.len() {
            3 => Some([d[0] * 17, d[1] * 17, d[2] * 17, 255]),
            4 => Some([d[0] * 17, d[1] * 17, d[2] * 17, d[3] * 17]),
            6 => Some([d[0] * 16 + d[1], d[2] * 16 + d[3], d[4] * 16 + d[5], 255]),
            8 => Some([d[0] * 16 + d[1], d[2] * 16 + d[3], d[4] * 16 + d[5], d[6] * 16 + d[7]]),
            _ => None,
        };
    }
    if let Some(a) = s.strip_prefix("rgba(").or_else(|| s.strip_prefix("rgb(")) {
        let inner = a.trim_end_matches(')');
        let p: Vec<f32> = inner
            .split(|c| c == ',' || c == ' ' || c == '/')
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(|x| match x.strip_suffix('%') {
                Some(pc) => pc.parse::<f32>().unwrap_or(0.0) * 2.55,
                None => x.parse::<f32>().unwrap_or(0.0),
            })
            .collect();
        if p.len() < 3 {
            return None;
        }
        let alpha = p.get(3).map_or(255.0, |a| if *a <= 1.0 { a * 255.0 } else { *a });
        return Some([p[0].clamp(0.0, 255.0) as u8, p[1].clamp(0.0, 255.0) as u8, p[2].clamp(0.0, 255.0) as u8, alpha.clamp(0.0, 255.0) as u8]);
    }
    let named = match s.as_str() {
        "black" => [0, 0, 0, 255],
        "white" => [255, 255, 255, 255],
        "red" => [255, 0, 0, 255],
        "green" => [0, 128, 0, 255],
        "lime" => [0, 255, 0, 255],
        "blue" => [0, 0, 255, 255],
        "yellow" => [255, 255, 0, 255],
        "orange" => [255, 165, 0, 255],
        "gray" | "grey" => [128, 128, 128, 255],
        "silver" => [192, 192, 192, 255],
        "cyan" | "aqua" => [0, 255, 255, 255],
        "magenta" | "fuchsia" => [255, 0, 255, 255],
        "transparent" => [0, 0, 0, 0],
        _ => return None,
    };
    Some(named)
}

pub(crate) struct Units {
    pub(crate) font: f32,
    pub(crate) vw: f32,
    pub(crate) vh: f32,
}

pub(crate) fn parse_len(v: &str, u: &Units) -> Option<Len> {
    let v = v.trim().to_ascii_lowercase();
    let num = |suffix: &str| v.strip_suffix(suffix).and_then(|n| n.trim().parse::<f32>().ok());
    if v == "0" {
        return Some(Len::Px(0.0));
    }
    if let Some(n) = num("px") {
        return Some(Len::Px(n));
    }
    if let Some(n) = num("%") {
        return Some(Len::Pct(n));
    }
    if let Some(n) = num("rem") {
        return Some(Len::Px(n * 16.0));
    }
    if let Some(n) = num("em") {
        return Some(Len::Px(n * u.font));
    }
    if let Some(n) = num("pt") {
        return Some(Len::Px(n * 4.0 / 3.0));
    }
    if let Some(n) = num("vw") {
        return Some(Len::Px(n * u.vw / 100.0));
    }
    if let Some(n) = num("vh") {
        return Some(Len::Px(n * u.vh / 100.0));
    }
    v.parse::<f32>().ok().map(Len::Px)
}

pub(crate) fn box_values(v: &str, u: &Units) -> ([f32; 4], bool) {
    let toks: Vec<&str> = v.split_whitespace().collect();
    let mut auto = false;
    let vals: Vec<f32> = toks
        .iter()
        .map(|t| {
            if *t == "auto" {
                auto = true;
                0.0
            } else {
                parse_len(t, u).map_or(0.0, |l| l.px(u.vw))
            }
        })
        .collect();
    let out = match vals.len() {
        1 => [vals[0]; 4],
        2 => [vals[0], vals[1], vals[0], vals[1]],
        3 => [vals[0], vals[1], vals[2], vals[1]],
        4 => [vals[0], vals[1], vals[2], vals[3]],
        _ => [0.0; 4],
    };
    (out, auto)
}

impl Style {
    pub(crate) fn apply(&mut self, prop: &str, val: &str, parent_font: f32, vw: f32, vh: f32) {
        let val = val.trim().trim_end_matches("!important").trim();
        let u = Units { font: parent_font, vw, vh };
        let own = Units { font: self.font_px, vw, vh };
        match prop {
            "color" => {
                if let Some(c) = parse_color(val) {
                    self.color = c;
                }
            }
            "background" | "background-color" => {
                if let Some(c) = val.split_whitespace().find_map(parse_color) {
                    self.bg = c;
                }
            }
            "font-size" => {
                let named = match val {
                    "small" => Some(13.0),
                    "medium" => Some(16.0),
                    "large" => Some(18.0),
                    "x-large" => Some(24.0),
                    "xx-large" => Some(32.0),
                    _ => None,
                };
                if let Some(px) = named {
                    self.font_px = px;
                } else if let Some(l) = parse_len(val, &u) {
                    self.font_px = l.px(parent_font).max(1.0);
                }
            }
            "font-weight" => {
                self.bold = matches!(val, "bold" | "bolder") || val.parse::<u32>().map_or(false, |w| w >= 600);
            }
            "text-align" => {
                self.align = match val {
                    "center" => 1,
                    "right" | "end" => 2,
                    _ => 0,
                };
            }
            "line-height" => {
                if let Ok(f) = val.parse::<f32>() {
                    self.line_h = f;
                } else if let Some(l) = parse_len(val, &own) {
                    self.line_h = l.px(self.font_px) / self.font_px;
                }
            }
            "margin" => {
                let (v, a) = box_values(val, &own);
                self.margin = v;
                self.margin_auto = a;
            }
            "margin-top" => self.margin[0] = parse_len(val, &own).map_or(0.0, |l| l.px(vw)),
            "margin-right" => {
                self.margin_auto |= val == "auto";
                self.margin[1] = parse_len(val, &own).map_or(0.0, |l| l.px(vw));
            }
            "margin-bottom" => self.margin[2] = parse_len(val, &own).map_or(0.0, |l| l.px(vw)),
            "margin-left" => {
                self.margin_auto |= val == "auto";
                self.margin[3] = parse_len(val, &own).map_or(0.0, |l| l.px(vw));
            }
            "padding" => self.padding = box_values(val, &own).0,
            "padding-top" => self.padding[0] = parse_len(val, &own).map_or(0.0, |l| l.px(vw)),
            "padding-right" => self.padding[1] = parse_len(val, &own).map_or(0.0, |l| l.px(vw)),
            "padding-bottom" => self.padding[2] = parse_len(val, &own).map_or(0.0, |l| l.px(vw)),
            "padding-left" => self.padding[3] = parse_len(val, &own).map_or(0.0, |l| l.px(vw)),
            "width" => self.width = if val == "auto" { None } else { parse_len(val, &own) },
            "height" => self.height = if val == "auto" { None } else { parse_len(val, &own) },
            "display" => {
                self.none = val == "none";
                self.inline = val == "inline";
                self.inline_block = val == "inline-block";
            }
            "visibility" => self.hidden = val == "hidden",
            "border-radius" => self.radius = parse_len(val, &own).map_or(0.0, |l| l.px(vw)),
            _ => {}
        }
    }
}
