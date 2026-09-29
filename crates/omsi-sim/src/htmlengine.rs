//! The built-in backend of `[htmltexture]`: a small HTML/CSS/JavaScript engine that needs no
//! browser and no extra dependency, so it runs the same on every desktop and on Android.
//!
//! What a page may use:
//! * HTML: nested elements, `<style>`, `<script>`, inline `style=""`, `id`, `class`, entities.
//! * CSS: selectors `tag`, `#id`, `.class`, `*`, compounds (`div.a#b`), descendant chains
//!   and `,` lists; `color`, `background(-color)`, `font-size`, `font-weight`, `text-align`,
//!   `line-height`, `margin*`, `padding*`, `width`, `height`, `display` (`none`, `inline`),
//!   `visibility`, `border-radius`. Units: `px`, `%`, `em`, `rem`, `pt`, `vw`, `vh`.
//!   Layout is block flow with wrapped inline text (no floats, no flexbox).
//! * JavaScript (ES5 plus arrow functions): `var/let/const`, functions, `if/for/while`,
//!   objects, arrays, `Math.*`, `parseInt/parseFloat/String/Number`,
//!   `document.getElementById/querySelector/body`, `element.textContent/innerText/className/id`,
//!   `element.style.*`, `element.setAttribute`.
//!
//! The page talks to the vehicle through `window.omsi`:
//! * the host calls `window.omsi.update({ num: {name: value}, str: {name: "text"} })` with the
//!   script variables that changed (all of them on the first call);
//! * the page calls `window.omsi.setVar(name, value)` to write a script variable back;
//! * the page calls `window.omsi.trigger(name)` to press a trigger of the vehicle's scripts
//!   (what a button in the cab does).
//!
//! A page can be operated: the app passes presses, releases and moves of the pointer on the
//! texture ([`HtmlRenderer::pointer`]). They become `pointerdown`/`mousedown`, `pointerup`/
//! `mouseup`, `click` and `mousemove` on the element under the pointer, which bubble up through
//! its parents. Listeners: the `onclick="..."` attribute (`event` is the event object), the
//! `element.onclick = f` property and `element.addEventListener("click", f)`; the event has
//! `type`, `x`, `y` (texture pixels), `target`, `stopPropagation()` and `preventDefault()`.
//! * `display:inline-block` (and `<button>`, which has a default look) lays boxes out in rows that
//!   wrap; a box without a `width` is as wide as its content. Use it for key pads and lists.
//! * More JavaScript for such pages: `setTimeout`/`setInterval`/`clear*`, `classList`,
//!   `createElement`/`appendChild`/`removeChild`/`remove`, `innerHTML` with markup, `getAttribute`,
//!   `parentNode`, `Object.keys`, `Array.forEach/map/filter/indexOf/includes/pop/shift/slice`,
//!   `String.split/replace`. Timers run when the page is drawn and use real time.

use crate::htmltex::{HtmlRenderer, PointerKind};
use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont, VariableFont};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const ROBOTO: &[u8] = include_bytes!("../../../assets/fonts/Roboto-VariableFont_wdth,wght.ttf");
const STEP_LIMIT: u32 = 400_000;
const DEPTH_LIMIT: u32 = 48;

// ───────────────────────────── DOM ─────────────────────────────

#[derive(Debug, Clone, Default)]
struct Node {
    tag: String,
    id: String,
    classes: Vec<String>,
    inline: Vec<(String, String)>,
    text: Option<String>,
    kids: Vec<usize>,
    parent: Option<usize>,
    /// Inline event attributes: `onclick="..."` is stored as `("click", "...")`.
    on: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
struct Simple {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
}

type Chain = Vec<Simple>;

#[derive(Debug, Clone)]
struct Rule {
    sels: Vec<Chain>,
    decls: Vec<(String, String)>,
    order: usize,
}

#[derive(Debug, Clone, Default)]
struct Dom {
    nodes: Vec<Node>,
    rules: Vec<Rule>,
    scripts: Vec<String>,
    body: usize,
}

const VOID: &[&str] = &["br", "img", "hr", "meta", "link", "input", "area", "base", "col", "source", "wbr"];

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        if let Some(e) = rest.find(';').filter(|e| *e <= 8) {
            let name = &rest[1..e];
            let rep = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{a0}'),
                _ => {
                    if let Some(h) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                        u32::from_str_radix(h, 16).ok().and_then(char::from_u32)
                    } else if let Some(d) = name.strip_prefix('#') {
                        d.parse::<u32>().ok().and_then(char::from_u32)
                    } else {
                        None
                    }
                }
            };
            if let Some(c) = rep {
                out.push(c);
                rest = &rest[e + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = false;
    for c in s.chars() {
        if c.is_whitespace() && c != '\u{a0}' {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out
}

fn parse_style_attr(s: &str) -> Vec<(String, String)> {
    s.split(';')
        .filter_map(|d| {
            let (k, v) = d.split_once(':')?;
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim().to_string());
            if k.is_empty() {
                None
            } else {
                Some((k, v))
            }
        })
        .collect()
}

impl Dom {
    fn parse(html: &str) -> Dom {
        let mut dom = Dom::default();
        dom.nodes.push(Node { tag: "#root".into(), ..Node::default() });
        let mut stack: Vec<usize> = vec![0];
        let mut css = String::new();
        let b = html.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'<' {
                let rest = &html[i..];
                if rest.starts_with("<!--") {
                    i += rest.find("-->").map(|e| e + 3).unwrap_or(rest.len());
                    continue;
                }
                if rest.starts_with("<!") || rest.starts_with("<?") {
                    i += rest.find('>').map(|e| e + 1).unwrap_or(rest.len());
                    continue;
                }
                if let Some(close) = rest.strip_prefix("</") {
                    let end = close.find('>').unwrap_or(close.len());
                    let name = close[..end].trim().to_ascii_lowercase();
                    if let Some(pos) = stack.iter().rposition(|&n| dom.nodes[n].tag == name) {
                        if pos > 0 {
                            stack.truncate(pos);
                        }
                    }
                    i += 2 + end + 1;
                    continue;
                }
                // an opening tag: find its end, honouring quotes
                let mut j = i + 1;
                let mut quote = 0u8;
                while j < b.len() {
                    let c = b[j];
                    if quote != 0 {
                        if c == quote {
                            quote = 0;
                        }
                    } else if c == b'"' || c == b'\'' {
                        quote = c;
                    } else if c == b'>' {
                        break;
                    }
                    j += 1;
                }
                let inner = &html[i + 1..j.min(html.len())];
                i = (j + 1).min(html.len());
                let self_closing = inner.ends_with('/');
                let inner = inner.trim_end_matches('/');
                let (name, attrs) = match inner.find(|c: char| c.is_whitespace()) {
                    Some(p) => (&inner[..p], &inner[p..]),
                    None => (inner, ""),
                };
                let name = name.to_ascii_lowercase();
                if name.is_empty() || !name.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
                    continue;
                }
                let mut node = Node { tag: name.clone(), parent: stack.last().copied(), ..Node::default() };
                for (k, v) in parse_attrs(attrs) {
                    match k.as_str() {
                        "id" => node.id = v,
                        "class" => node.classes = v.split_whitespace().map(str::to_string).collect(),
                        "style" => node.inline = parse_style_attr(&v),
                        e if e.len() > 2 && e.starts_with("on") => node.on.push((e[2..].to_string(), v)),
                        _ => {}
                    }
                }
                let idx = dom.nodes.len();
                let parent = *stack.last().unwrap();
                dom.nodes.push(node);
                dom.nodes[parent].kids.push(idx);
                if name == "style" || name == "script" {
                    let end_tag = format!("</{}", name);
                    let lower = html[i..].to_ascii_lowercase();
                    let end = lower.find(&end_tag).unwrap_or(lower.len());
                    let body = html[i..i + end].to_string();
                    if name == "style" {
                        css.push_str(&body);
                        css.push('\n');
                    } else {
                        dom.scripts.push(body);
                    }
                    i += end;
                    i += html[i..].find('>').map(|e| e + 1).unwrap_or(html.len() - i);
                    continue;
                }
                if !self_closing && !VOID.contains(&name.as_str()) {
                    stack.push(idx);
                }
            } else {
                let end = html[i..].find('<').map(|e| i + e).unwrap_or(html.len());
                let raw = decode_entities(&html[i..end]);
                i = end;
                let text = collapse_ws(&raw);
                if text.trim().is_empty() {
                    continue;
                }
                let parent = *stack.last().unwrap();
                let idx = dom.nodes.len();
                dom.nodes.push(Node { tag: "#text".into(), text: Some(text), parent: Some(parent), ..Node::default() });
                dom.nodes[parent].kids.push(idx);
            }
        }
        dom.rules = parse_css(&css);
        dom.body = dom.nodes.iter().position(|n| n.tag == "body").unwrap_or(0);
        dom
    }

    fn text_of(&self, idx: usize) -> String {
        let n = &self.nodes[idx];
        if let Some(t) = &n.text {
            return t.clone();
        }
        n.kids.iter().map(|&k| self.text_of(k)).collect::<Vec<_>>().join("")
    }

    fn set_text(&mut self, idx: usize, s: String) {
        let t = self.nodes.len();
        self.nodes.push(Node { tag: "#text".into(), text: Some(s), parent: Some(idx), ..Node::default() });
        self.nodes[idx].kids = vec![t];
    }

    /// A new element that hangs nowhere yet (`document.createElement`).
    fn create(&mut self, tag: &str) -> usize {
        self.nodes.push(Node { tag: tag.to_string(), ..Node::default() });
        self.nodes.len() - 1
    }

    fn detach(&mut self, idx: usize) {
        if let Some(p) = self.nodes[idx].parent.take() {
            self.nodes[p].kids.retain(|&k| k != idx);
        }
    }

    fn is_inside(&self, mut idx: usize, ancestor: usize) -> bool {
        loop {
            if idx == ancestor {
                return true;
            }
            match self.nodes[idx].parent {
                Some(p) => idx = p,
                None => return false,
            }
        }
    }

    /// `parent.appendChild(child)`; moving an element into itself or its own subtree is ignored.
    fn append(&mut self, parent: usize, child: usize) {
        if child == 0 || self.is_inside(parent, child) {
            return;
        }
        self.detach(child);
        self.nodes[child].parent = Some(parent);
        self.nodes[parent].kids.push(child);
    }

    fn copy_from(&mut self, src: &Dom, s: usize, parent: usize) {
        let mut n = src.nodes[s].clone();
        n.parent = Some(parent);
        n.kids.clear();
        let idx = self.nodes.len();
        self.nodes.push(n);
        self.nodes[parent].kids.push(idx);
        for &k in &src.nodes[s].kids {
            self.copy_from(src, k, idx);
        }
    }

    /// `element.innerHTML = markup`: the children are replaced by the parsed markup.
    /// (`<style>` and `<script>` in it are not run.)
    fn graft(&mut self, parent: usize, html: &str) {
        let frag = Dom::parse(&format!("<body>{html}</body>"));
        self.nodes[parent].kids.clear();
        for &k in &frag.nodes[frag.body].kids {
            self.copy_from(&frag, k, parent);
        }
    }

    fn by_id(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.text.is_none() && !n.id.is_empty() && n.id == id)
    }

    fn matches(&self, idx: usize, chain: &Chain) -> bool {
        let Some((last, rest)) = chain.split_last() else { return false };
        if !self.matches_simple(idx, last) {
            return false;
        }
        let mut cur = self.nodes[idx].parent;
        for s in rest.iter().rev() {
            loop {
                match cur {
                    Some(p) => {
                        cur = self.nodes[p].parent;
                        if self.matches_simple(p, s) {
                            break;
                        }
                    }
                    None => return false,
                }
            }
        }
        true
    }

    fn matches_simple(&self, idx: usize, s: &Simple) -> bool {
        let n = &self.nodes[idx];
        if n.text.is_some() {
            return false;
        }
        s.tag.as_ref().map_or(true, |t| *t == n.tag)
            && s.id.as_ref().map_or(true, |i| *i == n.id)
            && s.classes.iter().all(|c| n.classes.contains(c))
    }

    fn query(&self, sel: &str) -> Option<usize> {
        let chains: Vec<Chain> = sel.split(',').map(parse_chain).collect();
        (0..self.nodes.len()).find(|&i| chains.iter().any(|c| self.matches(i, c)))
    }
}

fn parse_attrs(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '=' {
            i += 1;
        }
        if start == i {
            i += 1;
            continue;
        }
        let key: String = chars[start..i].iter().collect::<String>().to_ascii_lowercase();
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let mut val = String::new();
        if i < chars.len() && chars[i] == '=' {
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
                let q = chars[i];
                i += 1;
                while i < chars.len() && chars[i] != q {
                    val.push(chars[i]);
                    i += 1;
                }
                i += 1;
            } else {
                while i < chars.len() && !chars[i].is_whitespace() {
                    val.push(chars[i]);
                    i += 1;
                }
            }
        }
        out.push((key, decode_entities(&val)));
    }
    out
}

fn parse_chain(sel: &str) -> Chain {
    sel.split_whitespace()
        .filter(|p| *p != ">" && *p != "+" && *p != "~")
        .map(|part| {
            let mut s = Simple::default();
            let mut cur = String::new();
            let mut mode = 't';
            let flush = |mode: char, cur: &mut String, s: &mut Simple| {
                if cur.is_empty() {
                    return;
                }
                let v = std::mem::take(cur);
                match mode {
                    't' if v != "*" => s.tag = Some(v.to_ascii_lowercase()),
                    '#' => s.id = Some(v),
                    '.' => s.classes.push(v),
                    _ => {}
                }
            };
            for c in part.chars() {
                if c == '#' || c == '.' {
                    flush(mode, &mut cur, &mut s);
                    mode = c;
                } else {
                    cur.push(c);
                }
            }
            flush(mode, &mut cur, &mut s);
            s
        })
        .collect()
}

fn specificity(chain: &Chain) -> u32 {
    chain
        .iter()
        .map(|s| (s.id.is_some() as u32) * 10_000 + (s.classes.len() as u32) * 100 + s.tag.is_some() as u32)
        .sum()
}

fn parse_css(src: &str) -> Vec<Rule> {
    // strip comments
    let mut clean = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(p) = rest.find("/*") {
        clean.push_str(&rest[..p]);
        rest = match rest[p..].find("*/") {
            Some(e) => &rest[p + e + 2..],
            None => "",
        };
    }
    clean.push_str(rest);
    let mut rules = Vec::new();
    let mut s = clean.as_str();
    while let Some(open) = s.find('{') {
        let head = s[..open].trim();
        let Some(close) = s[open..].find('}') else { break };
        let body = &s[open + 1..open + close];
        s = &s[open + close + 1..];
        if head.starts_with('@') {
            // an at-rule with a nested block: skip up to the matching close
            if body.contains('{') {
                if let Some(e) = s.find('}') {
                    s = &s[e + 1..];
                }
            }
            continue;
        }
        let sels: Vec<Chain> = head.split(',').map(parse_chain).filter(|c| !c.is_empty()).collect();
        let decls = parse_style_attr(body);
        if !sels.is_empty() {
            let order = rules.len();
            rules.push(Rule { sels, decls, order });
        }
    }
    rules
}

// ───────────────────────────── styles ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Len {
    Px(f32),
    Pct(f32),
}

impl Len {
    fn px(self, base: f32) -> f32 {
        match self {
            Len::Px(v) => v,
            Len::Pct(p) => base * p / 100.0,
        }
    }
}

#[derive(Debug, Clone)]
struct Style {
    color: [u8; 4],
    bg: [u8; 4],
    font_px: f32,
    bold: bool,
    align: u8,
    line_h: f32,
    hidden: bool,
    margin: [f32; 4],
    margin_auto: bool,
    padding: [f32; 4],
    width: Option<Len>,
    height: Option<Len>,
    none: bool,
    inline: bool,
    /// `display:inline-block`: a box that sits in a row with its neighbours.
    inline_block: bool,
    radius: f32,
    /// The element this style was computed for (what a text run belongs to when it is hit).
    node: usize,
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
    fn inherit(&self) -> Style {
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

fn parse_color(s: &str) -> Option<[u8; 4]> {
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

struct Units {
    font: f32,
    vw: f32,
    vh: f32,
}

fn parse_len(v: &str, u: &Units) -> Option<Len> {
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

fn box_values(v: &str, u: &Units) -> ([f32; 4], bool) {
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
    fn apply(&mut self, prop: &str, val: &str, parent_font: f32, vw: f32, vh: f32) {
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

// ───────────────────────────── layout ─────────────────────────────

struct LItem {
    text: String,
    px: f32,
    bold: bool,
    color: [u8; 4],
    dx: f32,
    w: f32,
    node: usize,
}

struct LLine {
    x: f32,
    y: f32,
    h: f32,
    items: Vec<LItem>,
}

enum Item {
    Block(LBox),
    Line(LLine),
}

struct LBox {
    rect: [f32; 4],
    mb: f32,
    st: Style,
    items: Vec<Item>,
    node: usize,
}

/// Move a laid-out box (and everything in it).
fn shift(b: &mut LBox, dx: f32, dy: f32) {
    b.rect[0] += dx;
    b.rect[1] += dy;
    for it in &mut b.items {
        match it {
            Item::Block(c) => shift(c, dx, dy),
            Item::Line(l) => {
                l.x += dx;
                l.y += dy;
            }
        }
    }
}

/// How wide the content of a box wants to be, counted from `cx` (its content's left edge).
fn natural_width(b: &LBox, cx: f32) -> f32 {
    let mut w = 0.0f32;
    for it in &b.items {
        match it {
            Item::Line(l) => {
                if let Some(li) = l.items.last() {
                    w = w.max(li.dx + li.w);
                }
            }
            Item::Block(c) => w = w.max(c.rect[0] - cx + c.rect[2] + c.st.margin[1]),
        }
    }
    w
}

/// The element under a point: the text run or box on top, `None` outside every box.
fn hit(b: &LBox, x: f32, y: f32) -> Option<usize> {
    if b.st.hidden {
        return None;
    }
    for it in b.items.iter().rev() {
        match it {
            Item::Block(c) => {
                if let Some(n) = hit(c, x, y) {
                    return Some(n);
                }
            }
            Item::Line(l) => {
                if y >= l.y && y < l.y + l.h {
                    for li in &l.items {
                        let x0 = l.x + li.dx;
                        if x >= x0 && x < x0 + li.w {
                            return Some(li.node);
                        }
                    }
                }
            }
        }
    }
    let [rx, ry, rw, rh] = b.rect;
    (x >= rx && x < rx + rw && y >= ry && y < ry + rh).then_some(b.node)
}

struct Layouter<'a> {
    dom: &'a Dom,
    reg: &'a FontRef<'static>,
    bold: &'a FontRef<'static>,
    vw: f32,
    vh: f32,
}

impl<'a> Layouter<'a> {
    fn tw(&self, s: &str, px: f32, bold: bool) -> f32 {
        let font = if bold { self.bold } else { self.reg };
        let sf = font.as_scaled(PxScale::from(px));
        let mut w = 0.0;
        let mut prev = None;
        for c in s.chars() {
            let id = font.glyph_id(c);
            if let Some(p) = prev {
                w += sf.kern(p, id);
            }
            w += sf.h_advance(id);
            prev = Some(id);
        }
        w
    }

    fn style_of(&self, idx: usize, ps: &Style) -> Style {
        let n = &self.dom.nodes[idx];
        let mut st = ps.inherit();
        let pf = ps.font_px;
        match n.tag.as_str() {
            "b" | "strong" => {
                st.inline = true;
                st.bold = true;
            }
            "span" | "i" | "em" | "a" | "small" | "u" | "label" | "code" => st.inline = true,
            "h1" => {
                st.font_px = pf * 2.0;
                st.bold = true;
                st.margin[0] = st.font_px * 0.67;
                st.margin[2] = st.font_px * 0.67;
            }
            "h2" => {
                st.font_px = pf * 1.5;
                st.bold = true;
                st.margin[0] = st.font_px * 0.83;
                st.margin[2] = st.font_px * 0.83;
            }
            "h3" => {
                st.font_px = pf * 1.17;
                st.bold = true;
                st.margin[0] = st.font_px;
                st.margin[2] = st.font_px;
            }
            "p" => {
                st.margin[0] = pf;
                st.margin[2] = pf;
            }
            "body" => st.margin = [8.0; 4],
            "button" => {
                st.inline_block = true;
                st.bg = [0xe0, 0xe0, 0xe0, 255];
                st.color = [0, 0, 0, 255];
                st.padding = [6.0, 12.0, 6.0, 12.0];
                st.align = 1;
                st.radius = 4.0;
            }
            "head" | "style" | "script" | "title" | "meta" | "link" => st.none = true,
            _ => {}
        }
        if n.tag == "small" {
            st.font_px = pf * 0.83;
        }
        let mut matched: Vec<(u32, usize, &Rule)> = self
            .dom
            .rules
            .iter()
            .filter_map(|r| {
                r.sels.iter().filter(|c| self.dom.matches(idx, c)).map(specificity).max().map(|s| (s, r.order, r))
            })
            .collect();
        matched.sort_by_key(|(s, o, _)| (*s, *o));
        for (_, _, r) in matched {
            for (k, v) in &r.decls {
                st.apply(k, v, pf, self.vw, self.vh);
            }
        }
        for (k, v) in &n.inline {
            st.apply(k, v, pf, self.vw, self.vh);
        }
        st.node = idx;
        st
    }

    fn inline_runs(&self, idx: usize, st: &Style, out: &mut Vec<(String, Style)>) {
        for &k in &self.dom.nodes[idx].kids {
            let kn = &self.dom.nodes[k];
            if let Some(t) = &kn.text {
                out.push((t.clone(), st.clone()));
                continue;
            }
            let cs = self.style_of(k, st);
            if cs.none {
                continue;
            }
            if kn.tag == "br" {
                out.push(("\n".into(), cs));
            } else if cs.inline {
                self.inline_runs(k, &cs, out);
            }
        }
    }

    fn finish_line(&self, items: &mut Vec<LItem>, lw: &mut f32, lh: &mut f32, out: &mut Vec<LLine>, cy: &mut f32, x: f32, w: f32, align: u8) {
        if let Some(l) = items.last_mut() {
            let t = l.text.trim_end().to_string();
            let nw = self.tw(&t, l.px, l.bold);
            *lw -= l.w - nw;
            l.w = nw;
            l.text = t;
        }
        let off = match align {
            1 => (w - *lw) / 2.0,
            2 => w - *lw,
            _ => 0.0,
        }
            .max(0.0);
        let h = *lh;
        out.push(LLine { x: x + off, y: *cy, h, items: std::mem::take(items) });
        *cy += h;
        *lw = 0.0;
        *lh = 0.0;
    }

    fn lines(&self, runs: &[(String, Style)], x: f32, y: f32, w: f32, align: u8) -> (Vec<LLine>, f32) {
        let mut out = Vec::new();
        let mut cy = y;
        let mut items: Vec<LItem> = Vec::new();
        let (mut lw, mut lh) = (0.0f32, 0.0f32);
        for (text, st) in runs {
            if text == "\n" {
                if lh == 0.0 {
                    lh = st.font_px * st.line_h;
                }
                self.finish_line(&mut items, &mut lw, &mut lh, &mut out, &mut cy, x, w, align);
                continue;
            }
            for tok in text.split_inclusive(' ') {
                let full = self.tw(tok, st.font_px, st.bold);
                let trimmed = self.tw(tok.trim_end(), st.font_px, st.bold);
                if !items.is_empty() && lw + trimmed > w + 0.01 {
                    self.finish_line(&mut items, &mut lw, &mut lh, &mut out, &mut cy, x, w, align);
                }
                if items.is_empty() && tok.trim().is_empty() {
                    continue;
                }
                items.push(LItem { text: tok.to_string(), px: st.font_px, bold: st.bold, color: st.color, dx: lw, w: full, node: st.node });
                lw += full;
                lh = lh.max(st.font_px * st.line_h);
            }
        }
        if !items.is_empty() {
            self.finish_line(&mut items, &mut lw, &mut lh, &mut out, &mut cy, x, w, align);
        }
        (out, cy - y)
    }

    fn build(&self, idx: usize, ps: &Style, x0: f32, y0: f32, avail: f32, pct_h: f32) -> LBox {
        self.build_w(idx, ps, x0, y0, avail, pct_h, None)
    }

    /// `force` fixes the content width (the row layout of inline blocks uses it).
    fn build_w(&self, idx: usize, ps: &Style, x0: f32, y0: f32, avail: f32, pct_h: f32, force: Option<f32>) -> LBox {
        let st = self.style_of(idx, ps);
        let [mt, mr, mb, ml] = st.margin;
        let [pt, pr, pb, pl] = st.padding;
        if force.is_none() && st.inline_block && st.width.is_none() {
            // shrink to fit: lay it out at full width, then narrow it to what the content used
            let auto = (avail - ml - mr - pl - pr).max(0.0);
            let probe = self.build_w(idx, ps, x0, y0, avail, pct_h, Some(auto));
            let nat = natural_width(&probe, x0 + ml + pl);
            return self.build_w(idx, ps, x0, y0, avail, pct_h, Some(nat.ceil().min(auto)));
        }
        let (outer_w, content_w) = match (force, st.width) {
            (Some(cw), _) => (cw + pl + pr, cw),
            (None, Some(l)) => {
                let cw = l.px(avail).max(0.0);
                (cw + pl + pr, cw)
            }
            (None, None) => {
                let ow = (avail - ml - mr).max(0.0);
                (ow, (ow - pl - pr).max(0.0))
            }
        };
        let bx = if st.margin_auto && st.width.is_some() { x0 + ((avail - outer_w) / 2.0).max(0.0) } else { x0 + ml };
        let by = y0 + mt;
        let cx = bx + pl;
        let mut cy = by + pt;
        let child_pct_h = match st.height {
            Some(l) => l.px(pct_h),
            None => pct_h,
        };
        let mut items = Vec::new();
        let mut runs: Vec<(String, Style)> = Vec::new();
        let flush = |runs: &mut Vec<(String, Style)>, cy: &mut f32, items: &mut Vec<Item>| {
            if runs.iter().any(|(t, _)| !t.trim().is_empty() || t == "\n") {
                let (lines, h) = self.lines(runs, cx, *cy, content_w, st.align);
                *cy += h;
                items.extend(lines.into_iter().map(Item::Line));
            }
            runs.clear();
        };
        // consecutive `inline-block` children are laid out in rows, wrapping at the content width
        let mut group: Vec<usize> = Vec::new();
        let place = |group: &mut Vec<usize>, cy: &mut f32, items: &mut Vec<Item>| {
            if group.is_empty() {
                return;
            }
            let mut boxes: Vec<LBox> =
                group.drain(..).map(|k| self.build_w(k, &st, 0.0, 0.0, content_w, child_pct_h, None)).collect();
            let occupied = |b: &LBox| b.st.margin[3] + b.rect[2] + b.st.margin[1];
            let mut i = 0;
            while i < boxes.len() {
                let (mut j, mut rw) = (i, 0.0f32);
                while j < boxes.len() {
                    let ow = occupied(&boxes[j]);
                    if j > i && rw + ow > content_w + 0.01 {
                        break;
                    }
                    rw += ow;
                    j += 1;
                }
                let off = match st.align {
                    1 => (content_w - rw) / 2.0,
                    2 => content_w - rw,
                    _ => 0.0,
                }
                    .max(0.0);
                let rh = boxes[i..j].iter().map(|b| b.st.margin[0] + b.rect[3] + b.st.margin[2]).fold(0.0, f32::max);
                let mut x = cx + off;
                for b in &mut boxes[i..j] {
                    let ow = occupied(b);
                    shift(b, x, *cy);
                    x += ow;
                }
                *cy += rh;
                i = j;
            }
            items.extend(boxes.into_iter().map(Item::Block));
        };
        for &k in &self.dom.nodes[idx].kids {
            let kn = &self.dom.nodes[k];
            if let Some(t) = &kn.text {
                place(&mut group, &mut cy, &mut items);
                runs.push((t.clone(), st.clone()));
                continue;
            }
            let cs = self.style_of(k, &st);
            if cs.none {
                continue;
            }
            if kn.tag == "br" {
                place(&mut group, &mut cy, &mut items);
                runs.push(("\n".into(), cs));
                continue;
            }
            if cs.inline_block {
                flush(&mut runs, &mut cy, &mut items);
                group.push(k);
                continue;
            }
            if cs.inline {
                place(&mut group, &mut cy, &mut items);
                self.inline_runs(k, &cs, &mut runs);
                continue;
            }
            place(&mut group, &mut cy, &mut items);
            flush(&mut runs, &mut cy, &mut items);
            let child = self.build(k, &st, cx, cy, content_w, child_pct_h);
            cy = child.rect[1] + child.rect[3] + child.mb;
            items.push(Item::Block(child));
        }
        place(&mut group, &mut cy, &mut items);
        flush(&mut runs, &mut cy, &mut items);
        let content_h = match st.height {
            Some(l) => l.px(pct_h),
            None => cy - (by + pt),
        };
        // a button with a fixed height keeps its label in the middle
        if self.dom.nodes[idx].tag == "button" && st.height.is_some() {
            let dy = ((content_h - (cy - (by + pt))) / 2.0).max(0.0);
            if dy > 0.0 {
                for it in &mut items {
                    match it {
                        Item::Block(c) => shift(c, 0.0, dy),
                        Item::Line(l) => l.y += dy,
                    }
                }
            }
        }
        let h = content_h + pt + pb;
        LBox { rect: [bx, by, outer_w, h], mb, st, items, node: idx }
    }
}

// ───────────────────────────── painting ─────────────────────────────

struct Canvas {
    w: u32,
    h: u32,
    px: Vec<u8>,
}

impl Canvas {
    fn blend(&mut self, x: i32, y: i32, c: [u8; 4], cov: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = ((y as u32 * self.w + x as u32) * 4) as usize;
        let sa = c[3] as f32 / 255.0 * cov.clamp(0.0, 1.0);
        if sa <= 0.0 {
            return;
        }
        let da = self.px[i + 3] as f32 / 255.0;
        let oa = sa + da * (1.0 - sa);
        for k in 0..3 {
            let v = (c[k] as f32 * sa + self.px[i + k] as f32 * da * (1.0 - sa)) / oa;
            self.px[i + k] = v.round().clamp(0.0, 255.0) as u8;
        }
        self.px[i + 3] = (oa * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    fn fill(&mut self, r: [f32; 4], c: [u8; 4], radius: f32) {
        let (x0, y0) = (r[0].round() as i32, r[1].round() as i32);
        let (x1, y1) = ((r[0] + r[2]).round() as i32, (r[1] + r[3]).round() as i32);
        let rad = radius.min(r[2] / 2.0).min(r[3] / 2.0).max(0.0);
        for y in y0.max(0)..y1.min(self.h as i32) {
            for x in x0.max(0)..x1.min(self.w as i32) {
                let mut cov = 1.0;
                if rad > 0.5 {
                    let px = x as f32 + 0.5;
                    let py = y as f32 + 0.5;
                    let cx = px.clamp(r[0] + rad, r[0] + r[2] - rad);
                    let cy = py.clamp(r[1] + rad, r[1] + r[3] - rad);
                    let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                    cov = (rad - d + 0.5).clamp(0.0, 1.0);
                }
                self.blend(x, y, c, cov);
            }
        }
    }
}

fn draw_text(cv: &mut Canvas, font: &FontRef<'static>, px: f32, x: f32, base_y: f32, color: [u8; 4], s: &str) {
    let sc = PxScale::from(px);
    let sf = font.as_scaled(sc);
    let mut cx = x;
    let mut prev = None;
    for ch in s.chars() {
        if ch == '\n' {
            continue;
        }
        let id = font.glyph_id(ch);
        if let Some(p) = prev {
            cx += sf.kern(p, id);
        }
        let g = id.with_scale_and_position(sc, point(cx, base_y));
        if let Some(o) = font.outline_glyph(g) {
            let b = o.px_bounds();
            let (bx, by) = (b.min.x as i32, b.min.y as i32);
            o.draw(|gx, gy, cov| cv.blend(bx + gx as i32, by + gy as i32, color, cov));
        }
        cx += sf.h_advance(id);
        prev = Some(id);
    }
}

fn paint(cv: &mut Canvas, lay: &Layouter, b: &LBox) {
    if !b.st.hidden && b.st.bg[3] > 0 {
        cv.fill(b.rect, b.st.bg, b.st.radius);
    }
    for it in &b.items {
        match it {
            Item::Block(c) => paint(cv, lay, c),
            Item::Line(l) => {
                if b.st.hidden {
                    continue;
                }
                for li in &l.items {
                    let font = if li.bold { lay.bold } else { lay.reg };
                    let sf = font.as_scaled(PxScale::from(li.px));
                    let base = l.y + (l.h - (sf.ascent() - sf.descent())) / 2.0 + sf.ascent();
                    draw_text(cv, font, li.px, l.x + li.dx, base, li.color, &li.text);
                }
            }
        }
    }
}

// ───────────────────────────── JavaScript ─────────────────────────────

type ObjRef = Arc<Mutex<HashMap<String, Val>>>;
type ArrRef = Arc<Mutex<Vec<Val>>>;
type Env = Arc<Mutex<Scope>>;

struct Scope {
    vars: HashMap<String, Val>,
    parent: Option<Env>,
}

struct FuncDef {
    params: Vec<String>,
    body: Vec<Stmt>,
}

struct Closure {
    def: Arc<FuncDef>,
    env: Env,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Nat {
    Round,
    Floor,
    Ceil,
    Abs,
    Min,
    Max,
    Sqrt,
    Pow,
    Trunc,
    Sin,
    Cos,
    ParseInt,
    ParseFloat,
    Str,
    Number,
    IsNaN,
    GetById,
    Query,
    SetVar,
    Trigger,
    Log,
    SetTimeout,
    SetInterval,
    ClearTimer,
    ObjectKeys,
    CreateEl,
}

#[derive(Clone)]
enum Val {
    Undef,
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Obj(ObjRef),
    Arr(ArrRef),
    Func(Arc<Closure>),
    Nat(Nat),
    Elem(usize),
    Style(usize),
    ClassList(usize),
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Id(String),
    P(String),
    Eof,
}

enum Expr {
    Num(f64),
    Str(String),
    Bool(bool),
    Null,
    Undef,
    Ident(String),
    Member(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Bin(String, Box<Expr>, Box<Expr>),
    Un(String, Box<Expr>),
    Assign(String, Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    Func(Arc<FuncDef>),
    Obj(Vec<(String, Expr)>),
    Arr(Vec<Expr>),
}

enum Stmt {
    Expr(Expr),
    Var(Vec<(String, Option<Expr>)>),
    Func(String, Arc<FuncDef>),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    For(Option<Box<Stmt>>, Option<Expr>, Option<Expr>, Box<Stmt>),
    While(Expr, Box<Stmt>),
    Return(Option<Expr>),
    Block(Vec<Stmt>),
    Break,
    Continue,
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    const PUNCT3: &[&str] = &["===", "!=="];
    const PUNCT2: &[&str] = &["==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "++", "--", "=>"];
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
        } else if ch == '/' && c.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < c.len() && !(c[i] == '*' && c[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if ch.is_ascii_digit() || (ch == '.' && c.get(i + 1).map_or(false, |d| d.is_ascii_digit())) {
            let s = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            if i < c.len() && (c[i] == 'e' || c[i] == 'E') {
                i += 1;
                if i < c.len() && (c[i] == '+' || c[i] == '-') {
                    i += 1;
                }
                while i < c.len() && c[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let t: String = c[s..i].iter().collect();
            out.push(Tok::Num(t.parse().map_err(|_| format!("bad number {t}"))?));
        } else if ch == '"' || ch == '\'' || ch == '`' {
            let q = ch;
            i += 1;
            let mut s = String::new();
            while i < c.len() && c[i] != q {
                if c[i] == '\\' && i + 1 < c.len() {
                    i += 1;
                    s.push(match c[i] {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        o => o,
                    });
                } else {
                    s.push(c[i]);
                }
                i += 1;
            }
            i += 1;
            out.push(Tok::Str(s));
        } else if ch.is_alphabetic() || ch == '_' || ch == '$' {
            let s = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_' || c[i] == '$') {
                i += 1;
            }
            out.push(Tok::Id(c[s..i].iter().collect()));
        } else {
            let rest: String = c[i..(i + 3).min(c.len())].iter().collect();
            if let Some(p) = PUNCT3.iter().find(|p| rest.starts_with(**p)) {
                out.push(Tok::P((*p).to_string()));
                i += 3;
            } else if let Some(p) = PUNCT2.iter().find(|p| rest.starts_with(**p)) {
                out.push(Tok::P((*p).to_string()));
                i += 2;
            } else if "{}()[];,.:?+-*/%<>=!".contains(ch) {
                out.push(Tok::P(ch.to_string()));
                i += 1;
            } else {
                return Err(format!("unexpected character {ch:?}"));
            }
        }
    }
    out.push(Tok::Eof);
    Ok(out)
}

const KEYWORDS: &[&str] = &["function", "return", "if", "else", "var", "let", "const", "for", "while", "typeof", "in", "break", "continue"];

struct Parser {
    t: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        self.t.get(self.i).unwrap_or(&Tok::Eof)
    }

    fn is_p(&self, p: &str) -> bool {
        matches!(self.peek(), Tok::P(q) if q == p)
    }

    fn is_id(&self, n: &str) -> bool {
        matches!(self.peek(), Tok::Id(q) if q == n)
    }

    fn eat(&mut self, p: &str) -> bool {
        if self.is_p(p) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, p: &str) -> Result<(), String> {
        if self.eat(p) {
            Ok(())
        } else {
            Err(format!("expected {p:?}, found {:?}", self.peek()))
        }
    }

    fn ident(&mut self) -> Result<String, String> {
        match self.peek().clone() {
            Tok::Id(n) => {
                self.i += 1;
                Ok(n)
            }
            t => Err(format!("expected a name, found {t:?}")),
        }
    }

    fn program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut out = Vec::new();
        while *self.peek() != Tok::Eof {
            out.push(self.stmt()?);
        }
        Ok(out)
    }

    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect("{")?;
        let mut out = Vec::new();
        while !self.is_p("}") {
            if *self.peek() == Tok::Eof {
                return Err("unclosed block".into());
            }
            out.push(self.stmt()?);
        }
        self.expect("}")?;
        Ok(out)
    }

    fn stmt(&mut self) -> Result<Stmt, String> {
        if self.is_p("{") {
            return Ok(Stmt::Block(self.block()?));
        }
        if self.eat(";") {
            return Ok(Stmt::Block(Vec::new()));
        }
        if let Tok::Id(k) = self.peek().clone() {
            match k.as_str() {
                "var" | "let" | "const" => {
                    self.i += 1;
                    let mut decls = Vec::new();
                    loop {
                        let name = self.ident()?;
                        let init = if self.eat("=") { Some(self.assign()?) } else { None };
                        decls.push((name, init));
                        if !self.eat(",") {
                            break;
                        }
                    }
                    self.eat(";");
                    return Ok(Stmt::Var(decls));
                }
                "function" if matches!(self.t.get(self.i + 1), Some(Tok::Id(_))) => {
                    self.i += 1;
                    let name = self.ident()?;
                    let def = self.func_rest()?;
                    return Ok(Stmt::Func(name, def));
                }
                "if" => {
                    self.i += 1;
                    self.expect("(")?;
                    let c = self.assign()?;
                    self.expect(")")?;
                    let a = Box::new(self.stmt()?);
                    let b = if self.is_id("else") {
                        self.i += 1;
                        Some(Box::new(self.stmt()?))
                    } else {
                        None
                    };
                    return Ok(Stmt::If(c, a, b));
                }
                "while" => {
                    self.i += 1;
                    self.expect("(")?;
                    let c = self.assign()?;
                    self.expect(")")?;
                    return Ok(Stmt::While(c, Box::new(self.stmt()?)));
                }
                "for" => {
                    self.i += 1;
                    self.expect("(")?;
                    let init = if self.eat(";") { None } else { Some(Box::new(self.stmt()?)) };
                    let cond = if self.is_p(";") { None } else { Some(self.assign()?) };
                    self.expect(";")?;
                    let upd = if self.is_p(")") { None } else { Some(self.assign()?) };
                    self.expect(")")?;
                    return Ok(Stmt::For(init, cond, upd, Box::new(self.stmt()?)));
                }
                "return" => {
                    self.i += 1;
                    let e = if self.is_p(";") || self.is_p("}") || *self.peek() == Tok::Eof { None } else { Some(self.assign()?) };
                    self.eat(";");
                    return Ok(Stmt::Return(e));
                }
                "break" => {
                    self.i += 1;
                    self.eat(";");
                    return Ok(Stmt::Break);
                }
                "continue" => {
                    self.i += 1;
                    self.eat(";");
                    return Ok(Stmt::Continue);
                }
                _ => {}
            }
        }
        let e = self.assign()?;
        self.eat(";");
        Ok(Stmt::Expr(e))
    }

    fn func_rest(&mut self) -> Result<Arc<FuncDef>, String> {
        self.expect("(")?;
        let mut params = Vec::new();
        while !self.is_p(")") {
            params.push(self.ident()?);
            if !self.eat(",") {
                break;
            }
        }
        self.expect(")")?;
        let body = self.block()?;
        Ok(Arc::new(FuncDef { params, body }))
    }

    fn is_arrow(&self) -> bool {
        match self.t.get(self.i) {
            Some(Tok::Id(n)) if !KEYWORDS.contains(&n.as_str()) => matches!(self.t.get(self.i + 1), Some(Tok::P(p)) if p == "=>"),
            Some(Tok::P(p)) if p == "(" => {
                let mut depth = 0;
                let mut j = self.i;
                while j < self.t.len() {
                    match &self.t[j] {
                        Tok::P(q) if q == "(" => depth += 1,
                        Tok::P(q) if q == ")" => {
                            depth -= 1;
                            if depth == 0 {
                                return matches!(self.t.get(j + 1), Some(Tok::P(r)) if r == "=>");
                            }
                        }
                        Tok::Eof => return false,
                        _ => {}
                    }
                    j += 1;
                }
                false
            }
            _ => false,
        }
    }

    fn arrow(&mut self) -> Result<Expr, String> {
        let mut params = Vec::new();
        if self.eat("(") {
            while !self.is_p(")") {
                params.push(self.ident()?);
                if !self.eat(",") {
                    break;
                }
            }
            self.expect(")")?;
        } else {
            params.push(self.ident()?);
        }
        self.expect("=>")?;
        let body = if self.is_p("{") { self.block()? } else { vec![Stmt::Return(Some(self.assign()?))] };
        Ok(Expr::Func(Arc::new(FuncDef { params, body })))
    }

    fn assign(&mut self) -> Result<Expr, String> {
        if self.is_arrow() {
            return self.arrow();
        }
        let lhs = self.cond()?;
        if let Tok::P(p) = self.peek().clone() {
            if matches!(p.as_str(), "=" | "+=" | "-=" | "*=" | "/=") {
                if !matches!(lhs, Expr::Ident(_) | Expr::Member(..)) {
                    return Err("invalid assignment target".into());
                }
                self.i += 1;
                let rhs = self.assign()?;
                return Ok(Expr::Assign(p, Box::new(lhs), Box::new(rhs)));
            }
        }
        Ok(lhs)
    }

    fn cond(&mut self) -> Result<Expr, String> {
        let c = self.bin(1)?;
        if self.eat("?") {
            let a = self.assign()?;
            self.expect(":")?;
            let b = self.assign()?;
            return Ok(Expr::Cond(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    fn prec(&self) -> Option<(String, u8)> {
        let (s, p) = match self.peek() {
            Tok::P(p) => (
                p.clone(),
                match p.as_str() {
                    "||" => 1,
                    "&&" => 2,
                    "==" | "!=" | "===" | "!==" => 3,
                    "<" | ">" | "<=" | ">=" => 4,
                    "+" | "-" => 5,
                    "*" | "/" | "%" => 6,
                    _ => return None,
                },
            ),
            Tok::Id(n) if n == "in" => ("in".to_string(), 4),
            _ => return None,
        };
        Some((s, p))
    }

    fn bin(&mut self, min: u8) -> Result<Expr, String> {
        let mut lhs = self.unary()?;
        while let Some((op, p)) = self.prec() {
            if p < min {
                break;
            }
            self.i += 1;
            let rhs = self.bin(p + 1)?;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if let Tok::P(p) = self.peek().clone() {
            if matches!(p.as_str(), "!" | "-" | "+") {
                self.i += 1;
                return Ok(Expr::Un(p, Box::new(self.unary()?)));
            }
        }
        if self.is_id("typeof") {
            self.i += 1;
            return Ok(Expr::Un("typeof".into(), Box::new(self.unary()?)));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.primary()?;
        loop {
            if self.eat(".") {
                let n = self.ident()?;
                e = Expr::Member(Box::new(e), Box::new(Expr::Str(n)));
            } else if self.eat("[") {
                let k = self.assign()?;
                self.expect("]")?;
                e = Expr::Member(Box::new(e), Box::new(k));
            } else if self.eat("(") {
                let mut args = Vec::new();
                while !self.is_p(")") {
                    args.push(self.assign()?);
                    if !self.eat(",") {
                        break;
                    }
                }
                self.expect(")")?;
                e = Expr::Call(Box::new(e), args);
            } else if self.is_p("++") || self.is_p("--") {
                let op = if self.is_p("++") { "+=" } else { "-=" };
                self.i += 1;
                e = Expr::Assign(op.into(), Box::new(e), Box::new(Expr::Num(1.0)));
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Tok::Num(n) => {
                self.i += 1;
                Ok(Expr::Num(n))
            }
            Tok::Str(s) => {
                self.i += 1;
                Ok(Expr::Str(s))
            }
            Tok::Id(n) => {
                self.i += 1;
                match n.as_str() {
                    "true" => Ok(Expr::Bool(true)),
                    "false" => Ok(Expr::Bool(false)),
                    "null" => Ok(Expr::Null),
                    "undefined" => Ok(Expr::Undef),
                    "function" => {
                        if matches!(self.peek(), Tok::Id(_)) {
                            self.i += 1;
                        }
                        Ok(Expr::Func(self.func_rest()?))
                    }
                    _ => Ok(Expr::Ident(n)),
                }
            }
            Tok::P(p) if p == "(" => {
                self.i += 1;
                let e = self.assign()?;
                self.expect(")")?;
                Ok(e)
            }
            Tok::P(p) if p == "[" => {
                self.i += 1;
                let mut items = Vec::new();
                while !self.is_p("]") {
                    items.push(self.assign()?);
                    if !self.eat(",") {
                        break;
                    }
                }
                self.expect("]")?;
                Ok(Expr::Arr(items))
            }
            Tok::P(p) if p == "{" => {
                self.i += 1;
                let mut props = Vec::new();
                while !self.is_p("}") {
                    let key = match self.peek().clone() {
                        Tok::Id(n) => n,
                        Tok::Str(s) => s,
                        Tok::Num(n) => fmt_num(n),
                        t => return Err(format!("bad object key {t:?}")),
                    };
                    self.i += 1;
                    self.expect(":")?;
                    props.push((key, self.assign()?));
                    if !self.eat(",") {
                        break;
                    }
                }
                self.expect("}")?;
                Ok(Expr::Obj(props))
            }
            t => Err(format!("unexpected {t:?}")),
        }
    }
}

fn fmt_num(n: f64) -> String {
    if n.is_nan() {
        "NaN".into()
    } else if n.is_infinite() {
        if n > 0.0 { "Infinity".into() } else { "-Infinity".into() }
    } else if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn to_str(v: &Val) -> String {
    match v {
        Val::Undef => "undefined".into(),
        Val::Null => "null".into(),
        Val::Bool(b) => b.to_string(),
        Val::Num(n) => fmt_num(*n),
        Val::Str(s) => s.clone(),
        Val::Arr(a) => a.lock().unwrap().iter().map(to_str).collect::<Vec<_>>().join(","),
        Val::Obj(_) | Val::Elem(_) | Val::Style(_) | Val::ClassList(_) => "[object Object]".into(),
        Val::Func(_) | Val::Nat(_) => "function".into(),
    }
}

fn to_num(v: &Val) -> f64 {
    match v {
        Val::Num(n) => *n,
        Val::Bool(b) => *b as u8 as f64,
        Val::Null => 0.0,
        Val::Str(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse().unwrap_or(f64::NAN)
            }
        }
        _ => f64::NAN,
    }
}

fn truthy(v: &Val) -> bool {
    match v {
        Val::Undef | Val::Null => false,
        Val::Bool(b) => *b,
        Val::Num(n) => *n != 0.0 && !n.is_nan(),
        Val::Str(s) => !s.is_empty(),
        _ => true,
    }
}

fn strict_eq(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::Undef, Val::Undef) | (Val::Null, Val::Null) => true,
        (Val::Bool(x), Val::Bool(y)) => x == y,
        (Val::Num(x), Val::Num(y)) => x == y,
        (Val::Str(x), Val::Str(y)) => x == y,
        (Val::Obj(x), Val::Obj(y)) => Arc::ptr_eq(x, y),
        (Val::Arr(x), Val::Arr(y)) => Arc::ptr_eq(x, y),
        (Val::Elem(x), Val::Elem(y)) => x == y,
        _ => false,
    }
}

fn loose_eq(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::Undef | Val::Null, Val::Undef | Val::Null) => true,
        (Val::Num(_) | Val::Str(_) | Val::Bool(_), Val::Num(_) | Val::Str(_) | Val::Bool(_)) => {
            if let (Val::Str(x), Val::Str(y)) = (a, b) {
                x == y
            } else {
                to_num(a) == to_num(b)
            }
        }
        _ => strict_eq(a, b),
    }
}

fn kebab(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn obj_of(props: &[(&str, Val)]) -> ObjRef {
    Arc::new(Mutex::new(props.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()))
}

enum Flow {
    Next,
    Ret(Val),
    Brk,
    Cont,
}

struct Timer {
    id: u32,
    due: f64,
    every: Option<f64>,
    f: Val,
}

struct Interp {
    dom: Dom,
    global: Env,
    window: ObjRef,
    events: Vec<(String, f32)>,
    /// Triggers the page has pressed (`omsi.trigger(name)`).
    triggers: Vec<String>,
    steps: u32,
    depth: u32,
    /// Listeners by (element, event name): `onclick = f` and `addEventListener("click", f)`.
    handlers: HashMap<(usize, String), Vec<Val>>,
    timers: Vec<Timer>,
    next_timer: u32,
    /// Seconds since the page started; the renderer sets it before it runs anything.
    now: f64,
    /// An error inside a callback of a built-in method (`forEach` ...), raised by the caller.
    pending_err: Option<String>,
}

impl Interp {
    fn new(dom: Dom) -> Interp {
        let global = Arc::new(Mutex::new(Scope { vars: HashMap::new(), parent: None }));
        let window = obj_of(&[("omsi", Val::Obj(obj_of(&[("setVar", Val::Nat(Nat::SetVar)), ("trigger", Val::Nat(Nat::Trigger))])))]);
        let it = Interp {
            dom,
            global,
            window,
            events: Vec::new(),
            triggers: Vec::new(),
            steps: 0,
            depth: 0,
            handlers: HashMap::new(),
            timers: Vec::new(),
            next_timer: 0,
            now: 0.0,
            pending_err: None,
        };
        let math = obj_of(&[
            ("round", Val::Nat(Nat::Round)),
            ("floor", Val::Nat(Nat::Floor)),
            ("ceil", Val::Nat(Nat::Ceil)),
            ("abs", Val::Nat(Nat::Abs)),
            ("min", Val::Nat(Nat::Min)),
            ("max", Val::Nat(Nat::Max)),
            ("sqrt", Val::Nat(Nat::Sqrt)),
            ("pow", Val::Nat(Nat::Pow)),
            ("trunc", Val::Nat(Nat::Trunc)),
            ("sin", Val::Nat(Nat::Sin)),
            ("cos", Val::Nat(Nat::Cos)),
            ("PI", Val::Num(std::f64::consts::PI)),
        ]);
        let body = it.dom.body;
        let doc = obj_of(&[
            ("getElementById", Val::Nat(Nat::GetById)),
            ("querySelector", Val::Nat(Nat::Query)),
            ("createElement", Val::Nat(Nat::CreateEl)),
            ("body", Val::Elem(body)),
        ]);
        let console = obj_of(&[("log", Val::Nat(Nat::Log)), ("warn", Val::Nat(Nat::Log)), ("error", Val::Nat(Nat::Log))]);
        {
            let mut g = it.global.lock().unwrap();
            g.vars.insert("Math".into(), Val::Obj(math));
            g.vars.insert("Object".into(), Val::Obj(obj_of(&[("keys", Val::Nat(Nat::ObjectKeys))])));
            g.vars.insert("setTimeout".into(), Val::Nat(Nat::SetTimeout));
            g.vars.insert("setInterval".into(), Val::Nat(Nat::SetInterval));
            g.vars.insert("clearTimeout".into(), Val::Nat(Nat::ClearTimer));
            g.vars.insert("clearInterval".into(), Val::Nat(Nat::ClearTimer));
            g.vars.insert("document".into(), Val::Obj(doc));
            g.vars.insert("console".into(), Val::Obj(console));
            g.vars.insert("window".into(), Val::Obj(it.window.clone()));
            g.vars.insert("parseInt".into(), Val::Nat(Nat::ParseInt));
            g.vars.insert("parseFloat".into(), Val::Nat(Nat::ParseFloat));
            g.vars.insert("String".into(), Val::Nat(Nat::Str));
            g.vars.insert("Number".into(), Val::Nat(Nat::Number));
            g.vars.insert("isNaN".into(), Val::Nat(Nat::IsNaN));
            g.vars.insert("NaN".into(), Val::Num(f64::NAN));
            g.vars.insert("Infinity".into(), Val::Num(f64::INFINITY));
        }
        it
    }

    fn run(&mut self, src: &str) -> Result<(), String> {
        let prog = Parser { t: lex(src)?, i: 0 }.program()?;
        self.steps = 0;
        let env = self.global.clone();
        self.exec_block(&prog, &env).map(|_| ())
    }

    fn lookup(&self, env: &Env, name: &str) -> Option<Val> {
        let mut cur = Some(env.clone());
        while let Some(e) = cur {
            let g = e.lock().unwrap();
            if let Some(v) = g.vars.get(name) {
                return Some(v.clone());
            }
            cur = g.parent.clone();
        }
        self.window.lock().unwrap().get(name).cloned()
    }

    fn assign_var(&self, env: &Env, name: &str, v: Val) {
        let mut cur = Some(env.clone());
        while let Some(e) = cur {
            let mut g = e.lock().unwrap();
            if g.vars.contains_key(name) {
                g.vars.insert(name.to_string(), v);
                return;
            }
            cur = g.parent.clone();
        }
        self.global.lock().unwrap().vars.insert(name.to_string(), v);
    }

    fn tick(&mut self) -> Result<(), String> {
        self.steps += 1;
        if self.steps > STEP_LIMIT {
            Err("script ran too long".into())
        } else {
            Ok(())
        }
    }

    fn exec_block(&mut self, stmts: &[Stmt], env: &Env) -> Result<Flow, String> {
        for s in stmts {
            if let Stmt::Func(n, d) = s {
                let f = Val::Func(Arc::new(Closure { def: d.clone(), env: env.clone() }));
                env.lock().unwrap().vars.insert(n.clone(), f);
            }
        }
        for s in stmts {
            match self.exec(s, env)? {
                Flow::Next => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Next)
    }

    fn exec(&mut self, s: &Stmt, env: &Env) -> Result<Flow, String> {
        self.tick()?;
        match s {
            Stmt::Expr(e) => {
                self.eval(e, env)?;
            }
            Stmt::Var(decls) => {
                for (n, init) in decls {
                    let v = match init {
                        Some(e) => self.eval(e, env)?,
                        None => Val::Undef,
                    };
                    env.lock().unwrap().vars.insert(n.clone(), v);
                }
            }
            Stmt::Func(..) => {}
            Stmt::If(c, a, b) => {
                if truthy(&self.eval(c, env)?) {
                    return self.exec(a, env);
                } else if let Some(b) = b {
                    return self.exec(b, env);
                }
            }
            Stmt::While(c, body) => {
                while truthy(&self.eval(c, env)?) {
                    self.tick()?;
                    match self.exec(body, env)? {
                        Flow::Ret(v) => return Ok(Flow::Ret(v)),
                        Flow::Brk => break,
                        _ => {}
                    }
                }
            }
            Stmt::For(init, cond, upd, body) => {
                if let Some(i) = init {
                    self.exec(i, env)?;
                }
                loop {
                    self.tick()?;
                    if let Some(c) = cond {
                        if !truthy(&self.eval(c, env)?) {
                            break;
                        }
                    }
                    match self.exec(body, env)? {
                        Flow::Ret(v) => return Ok(Flow::Ret(v)),
                        Flow::Brk => break,
                        _ => {}
                    }
                    if let Some(u) = upd {
                        self.eval(u, env)?;
                    }
                }
            }
            Stmt::Return(e) => {
                let v = match e {
                    Some(e) => self.eval(e, env)?,
                    None => Val::Undef,
                };
                return Ok(Flow::Ret(v));
            }
            Stmt::Block(b) => return self.exec_block(b, env),
            Stmt::Break => return Ok(Flow::Brk),
            Stmt::Continue => return Ok(Flow::Cont),
        }
        Ok(Flow::Next)
    }

    fn eval(&mut self, e: &Expr, env: &Env) -> Result<Val, String> {
        self.tick()?;
        Ok(match e {
            Expr::Num(n) => Val::Num(*n),
            Expr::Str(s) => Val::Str(s.clone()),
            Expr::Bool(b) => Val::Bool(*b),
            Expr::Null => Val::Null,
            Expr::Undef => Val::Undef,
            Expr::Ident(n) => self.lookup(env, n).unwrap_or(Val::Undef),
            Expr::Func(d) => Val::Func(Arc::new(Closure { def: d.clone(), env: env.clone() })),
            Expr::Obj(props) => {
                let mut m = HashMap::new();
                for (k, v) in props {
                    m.insert(k.clone(), self.eval(v, env)?);
                }
                Val::Obj(Arc::new(Mutex::new(m)))
            }
            Expr::Arr(items) => {
                let mut v = Vec::new();
                for i in items {
                    v.push(self.eval(i, env)?);
                }
                Val::Arr(Arc::new(Mutex::new(v)))
            }
            Expr::Member(o, k) => {
                let ov = self.eval(o, env)?;
                let kv = self.eval(k, env)?;
                self.get_prop(&ov, &to_str(&kv))
            }
            Expr::Cond(c, a, b) => {
                if truthy(&self.eval(c, env)?) {
                    self.eval(a, env)?
                } else {
                    self.eval(b, env)?
                }
            }
            Expr::Un(op, x) => {
                if op == "typeof" {
                    let v = self.eval(x, env)?;
                    return Ok(Val::Str(
                        match v {
                            Val::Undef => "undefined",
                            Val::Num(_) => "number",
                            Val::Str(_) => "string",
                            Val::Bool(_) => "boolean",
                            Val::Func(_) | Val::Nat(_) => "function",
                            _ => "object",
                        }
                            .into(),
                    ));
                }
                let v = self.eval(x, env)?;
                match op.as_str() {
                    "!" => Val::Bool(!truthy(&v)),
                    "-" => Val::Num(-to_num(&v)),
                    _ => Val::Num(to_num(&v)),
                }
            }
            Expr::Bin(op, a, b) => {
                if op == "&&" || op == "||" {
                    let l = self.eval(a, env)?;
                    return if (op == "&&") == truthy(&l) { self.eval(b, env) } else { Ok(l) };
                }
                let l = self.eval(a, env)?;
                let r = self.eval(b, env)?;
                self.binary(op, l, r)
            }
            Expr::Assign(op, target, rhs) => {
                let mut v = self.eval(rhs, env)?;
                if op != "=" {
                    let old = self.eval(target, env)?;
                    v = self.binary(&op[..1], old, v);
                }
                match target.as_ref() {
                    Expr::Ident(n) => self.assign_var(env, n, v.clone()),
                    Expr::Member(o, k) => {
                        let ov = self.eval(o, env)?;
                        let kv = self.eval(k, env)?;
                        self.set_prop(&ov, &to_str(&kv), v.clone())?;
                    }
                    _ => return Err("invalid assignment".into()),
                }
                v
            }
            Expr::Call(callee, args) => {
                let mut argv = Vec::with_capacity(args.len());
                for a in args {
                    argv.push(self.eval(a, env)?);
                }
                let (f, this) = match callee.as_ref() {
                    Expr::Member(o, k) => {
                        let ov = self.eval(o, env)?;
                        let kv = self.eval(k, env)?;
                        let key = to_str(&kv);
                        if let Some(r) = self.method(&ov, &key, &argv) {
                            return match self.pending_err.take() {
                                Some(e) => Err(e),
                                None => Ok(r),
                            };
                        }
                        (self.get_prop(&ov, &key), ov)
                    }
                    other => (self.eval(other, env)?, Val::Undef),
                };
                self.call(f, this, argv)?
            }
        })
    }

    fn binary(&self, op: &str, l: Val, r: Val) -> Val {
        match op {
            "+" => {
                if matches!(l, Val::Str(_)) || matches!(r, Val::Str(_)) {
                    Val::Str(format!("{}{}", to_str(&l), to_str(&r)))
                } else {
                    Val::Num(to_num(&l) + to_num(&r))
                }
            }
            "-" => Val::Num(to_num(&l) - to_num(&r)),
            "*" => Val::Num(to_num(&l) * to_num(&r)),
            "/" => Val::Num(to_num(&l) / to_num(&r)),
            "%" => Val::Num(to_num(&l) % to_num(&r)),
            "==" => Val::Bool(loose_eq(&l, &r)),
            "!=" => Val::Bool(!loose_eq(&l, &r)),
            "===" => Val::Bool(strict_eq(&l, &r)),
            "!==" => Val::Bool(!strict_eq(&l, &r)),
            "<" | ">" | "<=" | ">=" => {
                let ord = if let (Val::Str(a), Val::Str(b)) = (&l, &r) {
                    a.partial_cmp(b)
                } else {
                    to_num(&l).partial_cmp(&to_num(&r))
                };
                Val::Bool(match (op, ord) {
                    (_, None) => false,
                    ("<", Some(o)) => o.is_lt(),
                    (">", Some(o)) => o.is_gt(),
                    ("<=", Some(o)) => o.is_le(),
                    (_, Some(o)) => o.is_ge(),
                })
            }
            "in" => {
                let key = to_str(&l);
                Val::Bool(match &r {
                    Val::Obj(o) => o.lock().unwrap().contains_key(&key),
                    Val::Arr(a) => key.parse::<usize>().map_or(false, |i| i < a.lock().unwrap().len()),
                    _ => false,
                })
            }
            _ => Val::Undef,
        }
    }

    fn get_prop(&self, o: &Val, key: &str) -> Val {
        match o {
            Val::Obj(m) => m.lock().unwrap().get(key).cloned().unwrap_or(Val::Undef),
            Val::Arr(a) => {
                let a = a.lock().unwrap();
                if key == "length" {
                    Val::Num(a.len() as f64)
                } else {
                    key.parse::<usize>().ok().and_then(|i| a.get(i).cloned()).unwrap_or(Val::Undef)
                }
            }
            Val::Str(s) => {
                if key == "length" {
                    Val::Num(s.chars().count() as f64)
                } else {
                    Val::Undef
                }
            }
            Val::Elem(i) => {
                let n = &self.dom.nodes[*i];
                match key {
                    "textContent" | "innerText" | "innerHTML" => Val::Str(self.dom.text_of(*i)),
                    "className" => Val::Str(n.classes.join(" ")),
                    "id" => Val::Str(n.id.clone()),
                    "style" => Val::Style(*i),
                    "classList" => Val::ClassList(*i),
                    "parentNode" | "parentElement" => match n.parent {
                        Some(p) if p != 0 => Val::Elem(p),
                        _ => Val::Null,
                    },
                    _ => Val::Undef,
                }
            }
            _ => Val::Undef,
        }
    }

    fn set_prop(&mut self, o: &Val, key: &str, v: Val) -> Result<(), String> {
        match o {
            Val::Obj(m) => {
                m.lock().unwrap().insert(key.to_string(), v);
            }
            Val::Arr(a) => {
                if let Ok(i) = key.parse::<usize>() {
                    let mut a = a.lock().unwrap();
                    if i >= a.len() {
                        a.resize(i + 1, Val::Undef);
                    }
                    a[i] = v;
                }
            }
            Val::Elem(i) => match key {
                "innerHTML" => {
                    let markup = to_str(&v);
                    if markup.contains('<') {
                        self.dom.graft(*i, &markup);
                    } else {
                        self.dom.set_text(*i, markup);
                    }
                }
                "textContent" | "innerText" => self.dom.set_text(*i, to_str(&v)),
                "className" => self.dom.nodes[*i].classes = to_str(&v).split_whitespace().map(str::to_string).collect(),
                "id" => self.dom.nodes[*i].id = to_str(&v),
                k if k.len() > 2 && k.starts_with("on") => {
                    let ty = k[2..].to_string();
                    if matches!(v, Val::Func(_)) {
                        self.handlers.insert((*i, ty), vec![v]);
                    } else {
                        self.handlers.remove(&(*i, ty));
                    }
                }
                _ => {}
            },
            Val::Style(i) => {
                let prop = kebab(key);
                let inline = &mut self.dom.nodes[*i].inline;
                inline.retain(|(k, _)| *k != prop);
                inline.push((prop, to_str(&v)));
            }
            _ => {}
        }
        Ok(())
    }

    /// Methods of primitives and elements. `None` means: not a built-in method.
    fn method(&mut self, o: &Val, name: &str, args: &[Val]) -> Option<Val> {
        let arg_s = |i: usize| args.get(i).map(to_str).unwrap_or_default();
        let arg_n = |i: usize| args.get(i).map(to_num);
        match o {
            Val::Num(n) => match name {
                "toFixed" => Some(Val::Str(format!("{:.*}", arg_n(0).unwrap_or(0.0).clamp(0.0, 20.0) as usize, n))),
                "toString" => Some(Val::Str(fmt_num(*n))),
                _ => None,
            },
            Val::Str(s) => {
                let chars: Vec<char> = s.chars().collect();
                let idx = |v: Option<f64>, def: usize| -> usize {
                    match v {
                        None => def,
                        Some(x) if x < 0.0 => chars.len().saturating_sub((-x) as usize),
                        Some(x) => (x as usize).min(chars.len()),
                    }
                };
                match name {
                    "toUpperCase" => Some(Val::Str(s.to_uppercase())),
                    "toLowerCase" => Some(Val::Str(s.to_lowercase())),
                    "trim" => Some(Val::Str(s.trim().to_string())),
                    "toString" => Some(Val::Str(s.clone())),
                    "includes" => Some(Val::Bool(s.contains(&arg_s(0)))),
                    "startsWith" => Some(Val::Bool(s.starts_with(&arg_s(0)))),
                    "endsWith" => Some(Val::Bool(s.ends_with(&arg_s(0)))),
                    "indexOf" => Some(Val::Num(s.find(&arg_s(0)).map_or(-1.0, |b| s[..b].chars().count() as f64))),
                    "charAt" => Some(Val::Str(chars.get(arg_n(0).unwrap_or(0.0) as usize).map(|c| c.to_string()).unwrap_or_default())),
                    "repeat" => Some(Val::Str(s.repeat(arg_n(0).unwrap_or(0.0).clamp(0.0, 1000.0) as usize))),
                    "replace" => Some(Val::Str(s.replacen(&arg_s(0), &arg_s(1), 1))),
                    "split" => {
                        let sep = arg_s(0);
                        let parts: Vec<Val> = if args.is_empty() {
                            vec![Val::Str(s.clone())]
                        } else if sep.is_empty() {
                            chars.iter().map(|c| Val::Str(c.to_string())).collect()
                        } else {
                            s.split(&sep).map(|p| Val::Str(p.to_string())).collect()
                        };
                        Some(Val::Arr(Arc::new(Mutex::new(parts))))
                    }
                    "slice" | "substring" => {
                        let a = idx(arg_n(0), 0);
                        let b = idx(arg_n(1), chars.len());
                        Some(Val::Str(if a < b { chars[a..b].iter().collect() } else { String::new() }))
                    }
                    "padStart" | "padEnd" => {
                        let want = arg_n(0).unwrap_or(0.0).max(0.0) as usize;
                        let pad = if args.len() > 1 { arg_s(1) } else { " ".into() };
                        let mut fill = String::new();
                        if !pad.is_empty() {
                            let mut it = pad.chars().cycle();
                            for _ in chars.len()..want {
                                fill.push(it.next().unwrap());
                            }
                        }
                        Some(Val::Str(if name == "padStart" { format!("{fill}{s}") } else { format!("{s}{fill}") }))
                    }
                    _ => None,
                }
            }
            Val::Arr(a) => match name {
                "push" => {
                    let mut a = a.lock().unwrap();
                    a.extend(args.iter().cloned());
                    Some(Val::Num(a.len() as f64))
                }
                "join" => {
                    let sep = if args.is_empty() { ",".to_string() } else { arg_s(0) };
                    Some(Val::Str(a.lock().unwrap().iter().map(to_str).collect::<Vec<_>>().join(&sep)))
                }
                "forEach" | "map" | "filter" => {
                    let f = args.first().cloned().unwrap_or(Val::Undef);
                    let items: Vec<Val> = a.lock().unwrap().clone();
                    let mut out = Vec::new();
                    for (i, v) in items.into_iter().enumerate() {
                        match self.call(f.clone(), Val::Undef, vec![v.clone(), Val::Num(i as f64)]) {
                            Ok(r) => match name {
                                "map" => out.push(r),
                                "filter" => {
                                    if truthy(&r) {
                                        out.push(v)
                                    }
                                }
                                _ => {}
                            },
                            Err(e) => {
                                self.pending_err = Some(e);
                                return Some(Val::Undef);
                            }
                        }
                    }
                    Some(if name == "forEach" { Val::Undef } else { Val::Arr(Arc::new(Mutex::new(out))) })
                }
                "indexOf" | "includes" => {
                    let want = args.first().cloned().unwrap_or(Val::Undef);
                    let pos = a.lock().unwrap().iter().position(|x| strict_eq(x, &want));
                    Some(if name == "indexOf" { Val::Num(pos.map_or(-1.0, |p| p as f64)) } else { Val::Bool(pos.is_some()) })
                }
                "pop" => Some(a.lock().unwrap().pop().unwrap_or(Val::Undef)),
                "shift" => {
                    let mut a = a.lock().unwrap();
                    Some(if a.is_empty() { Val::Undef } else { a.remove(0) })
                }
                "slice" => {
                    let v = a.lock().unwrap();
                    let len = v.len();
                    let at = |x: Option<f64>, def: usize| match x {
                        None => def,
                        Some(x) if x < 0.0 => len.saturating_sub((-x) as usize),
                        Some(x) => (x as usize).min(len),
                    };
                    let (from, to) = (at(arg_n(0), 0), at(arg_n(1), len));
                    Some(Val::Arr(Arc::new(Mutex::new(if from < to { v[from..to].to_vec() } else { Vec::new() }))))
                }
                _ => None,
            },
            Val::Obj(m) if name == "hasOwnProperty" => Some(Val::Bool(m.lock().unwrap().contains_key(&arg_s(0)))),
            Val::Obj(m) if name == "stopPropagation" => {
                m.lock().unwrap().insert("cancelBubble".into(), Val::Bool(true));
                Some(Val::Undef)
            }
            Val::Obj(_) if name == "preventDefault" => Some(Val::Undef),
            Val::Elem(i) if name == "addEventListener" => {
                if let Some(f @ Val::Func(_)) = args.get(1) {
                    self.handlers.entry((*i, arg_s(0))).or_default().push(f.clone());
                }
                Some(Val::Undef)
            }
            Val::Elem(i) if name == "getAttribute" => {
                let n = &self.dom.nodes[*i];
                Some(match arg_s(0).as_str() {
                    "id" => Val::Str(n.id.clone()),
                    "class" => Val::Str(n.classes.join(" ")),
                    _ => Val::Null,
                })
            }
            Val::Elem(i) if name == "appendChild" => {
                if let Some(Val::Elem(c)) = args.first() {
                    self.dom.append(*i, *c);
                }
                Some(args.first().cloned().unwrap_or(Val::Undef))
            }
            Val::Elem(i) if name == "removeChild" => {
                if let Some(Val::Elem(c)) = args.first() {
                    if self.dom.nodes[*c].parent == Some(*i) {
                        self.dom.detach(*c);
                    }
                }
                Some(args.first().cloned().unwrap_or(Val::Undef))
            }
            Val::Elem(i) if name == "remove" => {
                self.dom.detach(*i);
                Some(Val::Undef)
            }
            Val::ClassList(i) => {
                let cls = arg_s(0);
                let n = &mut self.dom.nodes[*i];
                let has = n.classes.contains(&cls);
                match name {
                    "add" => {
                        if !has && !cls.is_empty() {
                            n.classes.push(cls);
                        }
                        Some(Val::Undef)
                    }
                    "remove" => {
                        n.classes.retain(|c| *c != cls);
                        Some(Val::Undef)
                    }
                    "toggle" => {
                        let want = args.get(1).map(truthy).unwrap_or(!has);
                        if want && !has && !cls.is_empty() {
                            n.classes.push(cls);
                        } else if !want && has {
                            n.classes.retain(|c| *c != cls);
                        }
                        Some(Val::Bool(want))
                    }
                    "contains" => Some(Val::Bool(has)),
                    _ => None,
                }
            }
            Val::Elem(i) if name == "setAttribute" => {
                let (k, v) = (arg_s(0), arg_s(1));
                let n = &mut self.dom.nodes[*i];
                match k.as_str() {
                    "class" => n.classes = v.split_whitespace().map(str::to_string).collect(),
                    "id" => n.id = v,
                    "style" => n.inline = parse_style_attr(&v),
                    _ => {}
                }
                Some(Val::Undef)
            }
            _ => None,
        }
    }

    fn call(&mut self, f: Val, _this: Val, args: Vec<Val>) -> Result<Val, String> {
        match f {
            Val::Nat(n) => Ok(self.call_nat(n, &args)),
            Val::Func(c) => {
                self.depth += 1;
                if self.depth > DEPTH_LIMIT {
                    self.depth -= 1;
                    return Err("call stack too deep".into());
                }
                self.tick()?;
                let scope = Arc::new(Mutex::new(Scope { vars: HashMap::new(), parent: Some(c.env.clone()) }));
                {
                    let mut s = scope.lock().unwrap();
                    for (i, p) in c.def.params.iter().enumerate() {
                        s.vars.insert(p.clone(), args.get(i).cloned().unwrap_or(Val::Undef));
                    }
                }
                let r = self.exec_block(&c.def.body, &scope);
                self.depth -= 1;
                match r? {
                    Flow::Ret(v) => Ok(v),
                    _ => Ok(Val::Undef),
                }
            }
            _ => Err("not a function".into()),
        }
    }

    fn call_nat(&mut self, n: Nat, args: &[Val]) -> Val {
        let a = |i: usize| args.get(i).map_or(f64::NAN, to_num);
        match n {
            Nat::Round => Val::Num((a(0) + 0.5).floor()),
            Nat::Floor => Val::Num(a(0).floor()),
            Nat::Ceil => Val::Num(a(0).ceil()),
            Nat::Abs => Val::Num(a(0).abs()),
            Nat::Trunc => Val::Num(a(0).trunc()),
            Nat::Sqrt => Val::Num(a(0).sqrt()),
            Nat::Sin => Val::Num(a(0).sin()),
            Nat::Cos => Val::Num(a(0).cos()),
            Nat::Pow => Val::Num(a(0).powf(a(1))),
            Nat::Min => Val::Num(args.iter().map(to_num).fold(f64::INFINITY, f64::min)),
            Nat::Max => Val::Num(args.iter().map(to_num).fold(f64::NEG_INFINITY, f64::max)),
            Nat::ParseInt => {
                let s = args.first().map(to_str).unwrap_or_default();
                let t = s.trim();
                let end = t.char_indices().find(|(i, c)| !(c.is_ascii_digit() || (*i == 0 && (*c == '-' || *c == '+')))).map_or(t.len(), |(i, _)| i);
                Val::Num(t[..end].parse::<f64>().unwrap_or(f64::NAN))
            }
            Nat::ParseFloat => {
                let s = args.first().map(to_str).unwrap_or_default();
                let t = s.trim();
                let end = t
                    .char_indices()
                    .find(|(i, c)| !(c.is_ascii_digit() || *c == '.' || (*i == 0 && (*c == '-' || *c == '+'))))
                    .map_or(t.len(), |(i, _)| i);
                Val::Num(t[..end].parse::<f64>().unwrap_or(f64::NAN))
            }
            Nat::Str => Val::Str(args.first().map(to_str).unwrap_or_default()),
            Nat::Number => Val::Num(if args.is_empty() { 0.0 } else { a(0) }),
            Nat::IsNaN => Val::Bool(a(0).is_nan()),
            Nat::GetById => match args.first().map(to_str).and_then(|id| self.dom.by_id(&id)) {
                Some(i) => Val::Elem(i),
                None => Val::Null,
            },
            Nat::Query => match args.first().map(to_str).and_then(|s| self.dom.query(&s)) {
                Some(i) => Val::Elem(i),
                None => Val::Null,
            },
            Nat::SetVar => {
                if let Some(name) = args.first().map(to_str) {
                    let v = match args.get(1) {
                        Some(Val::Bool(b)) => *b as u8 as f32,
                        Some(v) => to_num(v) as f32,
                        None => 0.0,
                    };
                    if v.is_finite() {
                        log::info!("htmltexture: page calls setVar({name}, {v})");
                        self.events.push((name, v));
                    } else {
                        log::info!("htmltexture: setVar({name}) ignored, the value is not a finite number");
                    }
                }
                Val::Undef
            }
            Nat::Trigger => {
                if let Some(name) = args.first().map(to_str).filter(|n| !n.is_empty()) {
                    log::info!("htmltexture: page presses trigger {name}");
                    self.triggers.push(name);
                }
                Val::Undef
            }
            Nat::Log => {
                log::info!("htmltexture console: {}", args.iter().map(to_str).collect::<Vec<_>>().join(" "));
                Val::Undef
            }
            Nat::SetTimeout | Nat::SetInterval => {
                let f = args.first().cloned().unwrap_or(Val::Undef);
                if !matches!(f, Val::Func(_)) || self.timers.len() >= 64 {
                    return Val::Undef;
                }
                let ms = a(1);
                let ms = if ms.is_finite() { ms.max(0.0) } else { 0.0 };
                self.next_timer += 1;
                let id = self.next_timer;
                let every = (n == Nat::SetInterval).then(|| (ms / 1000.0).max(0.016));
                self.timers.push(Timer { id, due: self.now + ms / 1000.0, every, f });
                Val::Num(id as f64)
            }
            Nat::ClearTimer => {
                let id = a(0);
                self.timers.retain(|t| t.id as f64 != id);
                Val::Undef
            }
            Nat::ObjectKeys => {
                let mut keys: Vec<String> = match args.first() {
                    Some(Val::Obj(m)) => m.lock().unwrap().keys().cloned().collect(),
                    _ => Vec::new(),
                };
                keys.sort();
                Val::Arr(Arc::new(Mutex::new(keys.into_iter().map(Val::Str).collect())))
            }
            Nat::CreateEl => {
                let tag = args.first().map(to_str).unwrap_or_default().to_ascii_lowercase();
                Val::Elem(self.dom.create(&tag))
            }
        }
    }
}

// ───────────────────────────── the renderer ─────────────────────────────

/// The engine as an [`HtmlRenderer`].
pub struct EngineRenderer {
    width: u32,
    height: u32,
    js: Interp,
    dirty: bool,
    warned: bool,
    start: std::time::Instant,
    /// A press is on the page and its release has not come yet (a click needs both).
    pressed: bool,
}

impl EngineRenderer {
    pub fn new(width: u32, height: u32, html: &str) -> EngineRenderer {
        let dom = Dom::parse(html);
        let scripts = dom.scripts.clone();
        log::info!(
            "htmltexture: page parsed: {} nodes, {} css rules, {} script(s), {}x{}",
            dom.nodes.len(),
            dom.rules.len(),
            scripts.len(),
            width,
            height
        );
        let mut js = Interp::new(dom);
        let mut warned = false;
        for (n, s) in scripts.iter().enumerate() {
            match js.run(s) {
                Ok(()) => log::info!("htmltexture: script {n} ran ({} steps)", js.steps),
                Err(e) => {
                    log::warn!("htmltexture: script error: {e}");
                    warned = true;
                }
            }
        }
        EngineRenderer {
            width: width.max(1),
            height: height.max(1),
            js,
            dirty: true,
            warned,
            start: std::time::Instant::now(),
            pressed: false,
        }
    }

    fn clock(&mut self) {
        self.js.now = self.start.elapsed().as_secs_f64();
    }

    /// Run the timers that are due. True when one ran (the page probably changed).
    fn run_timers(&mut self) -> bool {
        let now = self.js.now;
        let due: Vec<(u32, Val)> = self.js.timers.iter().filter(|t| t.due <= now).map(|t| (t.id, t.f.clone())).collect();
        if due.is_empty() {
            return false;
        }
        // an interval is set up again before it runs, so it can stop itself
        self.js.timers.retain_mut(|t| {
            if t.due > now {
                return true;
            }
            match t.every {
                Some(e) => {
                    t.due = now + e;
                    true
                }
                None => false,
            }
        });
        for (id, f) in due {
            self.js.steps = 0;
            if let Err(e) = self.js.call(f, Val::Undef, Vec::new()) {
                log::warn!("htmltexture: timer {id} failed: {e}");
            }
        }
        true
    }

    /// The root (`html`) style is what the body inherits from.
    fn root_style(&self, lay: &Layouter) -> Style {
        let dom = &self.js.dom;
        match dom.nodes[dom.body].parent.filter(|&p| p != 0) {
            Some(h) => lay.style_of(h, &Style::default()),
            None => Style::default(),
        }
    }

    /// The element at a point of the texture (pixels); the body when nothing is there.
    fn hit_node(&self, x: f32, y: f32) -> usize {
        let body = self.js.dom.body;
        let Ok(reg) = FontRef::try_from_slice(ROBOTO) else { return body };
        let mut bold = reg.clone();
        bold.set_variation(b"wght", 700.0);
        let lay = Layouter { dom: &self.js.dom, reg: &reg, bold: &bold, vw: self.width as f32, vh: self.height as f32 };
        let root = self.root_style(&lay);
        let b = lay.build(body, &root, 0.0, 0.0, self.width as f32, self.height as f32);
        hit(&b, x, y).unwrap_or(body)
    }

    /// Fire `ty` on `node` and let it bubble up through its parents.
    fn dispatch(&mut self, node: usize, ty: &str, x: f32, y: f32) {
        let ev = obj_of(&[
            ("type", Val::Str(ty.to_string())),
            ("x", Val::Num(x as f64)),
            ("y", Val::Num(y as f64)),
            ("clientX", Val::Num(x as f64)),
            ("clientY", Val::Num(y as f64)),
            ("target", Val::Elem(node)),
        ]);
        self.js.global.lock().unwrap().vars.insert("event".into(), Val::Obj(ev.clone()));
        let mut cur = Some(node);
        while let Some(n) = cur {
            let src = self.js.dom.nodes[n].on.iter().find(|(k, _)| k == ty).map(|(_, s)| s.clone());
            if let Some(src) = src {
                if let Err(e) = self.js.run(&src) {
                    log::warn!("htmltexture: on{ty} handler failed: {e}");
                }
            }
            let listeners = self.js.handlers.get(&(n, ty.to_string())).cloned().unwrap_or_default();
            for f in listeners {
                self.js.steps = 0;
                if let Err(e) = self.js.call(f, Val::Undef, vec![Val::Obj(ev.clone())]) {
                    log::warn!("htmltexture: {ty} listener failed: {e}");
                }
            }
            if ev.lock().unwrap().contains_key("cancelBubble") {
                break;
            }
            cur = self.js.dom.nodes[n].parent.filter(|&p| p != 0);
        }
    }

    fn call_update(&mut self, num: &[(String, f32)], strs: &[(String, String)]) {
        let omsi = self.js.window.lock().unwrap().get("omsi").cloned();
        let Some(Val::Obj(omsi)) = omsi else {
            log::info!("htmltexture: the page has no window.omsi");
            return;
        };
        let update = omsi.lock().unwrap().get("update").cloned();
        let Some(update @ Val::Func(_)) = update else {
            log::info!("htmltexture: the page has no window.omsi.update function");
            return;
        };
        log::info!("htmltexture: window.omsi.update with {} numeric and {} string variable(s)", num.len(), strs.len());
        let nums: HashMap<String, Val> = num.iter().map(|(k, v)| (k.clone(), Val::Num(*v as f64))).collect();
        let strv: HashMap<String, Val> = strs.iter().map(|(k, v)| (k.clone(), Val::Str(v.clone()))).collect();
        let arg = obj_of(&[("num", Val::Obj(Arc::new(Mutex::new(nums)))), ("str", Val::Obj(Arc::new(Mutex::new(strv))))]);
        self.js.steps = 0;
        let result = self.js.call(update, Val::Obj(omsi), vec![Val::Obj(arg)]);
        log::info!("htmltexture: window.omsi.update took {} steps", self.js.steps);
        if let Err(e) = result {
            if !self.warned {
                log::warn!("htmltexture: omsi.update failed: {e}");
                self.warned = true;
            }
        }
    }

    fn render(&self) -> Vec<u8> {
        let started = std::time::Instant::now();
        let mut cv = Canvas { w: self.width, h: self.height, px: vec![0; (self.width * self.height * 4) as usize] };
        let Ok(reg) = FontRef::try_from_slice(ROBOTO) else { return cv.px };
        let mut bold = reg.clone();
        bold.set_variation(b"wght", 700.0);
        let lay = Layouter { dom: &self.js.dom, reg: &reg, bold: &bold, vw: self.width as f32, vh: self.height as f32 };
        let body = self.js.dom.body;
        let root = self.root_style(&lay);
        if root.bg[3] > 0 {
            cv.fill([0.0, 0.0, self.width as f32, self.height as f32], root.bg, 0.0);
        }
        let b = lay.build(body, &root, 0.0, 0.0, self.width as f32, self.height as f32);
        if b.st.bg[3] > 0 {
            cv.fill([0.0, 0.0, self.width as f32, self.height as f32], b.st.bg, 0.0);
        }
        let mut b = b;
        b.st.bg = [0, 0, 0, 0];
        paint(&mut cv, &lay, &b);
        log::info!("htmltexture: rendered {}x{} in {:?}", self.width, self.height, started.elapsed());
        cv.px
    }

    #[cfg(test)]
    fn text_of(&self, id: &str) -> Option<String> {
        self.js.dom.by_id(id).map(|i| self.js.dom.text_of(i))
    }
}

impl HtmlRenderer for EngineRenderer {
    fn set_vars(&mut self, num: &[(String, f32)], strs: &[(String, String)]) {
        self.clock();
        self.call_update(num, strs);
        self.dirty = true;
    }

    fn pointer(&mut self, x: f32, y: f32, kind: PointerKind) {
        self.clock();
        let node = self.hit_node(x, y);
        match kind {
            PointerKind::Down => {
                self.pressed = true;
                self.dispatch(node, "pointerdown", x, y);
                self.dispatch(node, "mousedown", x, y);
            }
            PointerKind::Up => {
                self.dispatch(node, "pointerup", x, y);
                self.dispatch(node, "mouseup", x, y);
                if std::mem::take(&mut self.pressed) {
                    self.dispatch(node, "click", x, y);
                }
            }
            PointerKind::Move => self.dispatch(node, "mousemove", x, y),
        }
        self.dirty = true;
    }

    fn poll_frame(&mut self) -> Option<Vec<u8>> {
        self.clock();
        if self.run_timers() {
            self.dirty = true;
        }
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        Some(self.render())
    }

    fn take_events(&mut self) -> Vec<(String, f32)> {
        std::mem::take(&mut self.js.events)
    }

    fn take_triggers(&mut self) -> Vec<String> {
        std::mem::take(&mut self.js.triggers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IBIS: &str = include_str!("../../../docs/examples/htmltexture/ibis.html");

    fn px(frame: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * w + x) * 4) as usize;
        [frame[i], frame[i + 1], frame[i + 2], frame[i + 3]]
    }

    #[test]
    fn the_ibis_example_shows_the_speed() {
        let mut r = EngineRenderer::new(800, 480, IBIS);
        r.set_vars(&[("velocity_kmh".into(), 42.4)], &[("next_stop".into(), "Hauptbahnhof".into())]);
        assert_eq!(r.text_of("speed").as_deref(), Some("42"));
        assert_eq!(r.text_of("stop").as_deref(), Some("Hauptbahnhof"));
        let a = r.poll_frame().unwrap();
        assert_eq!(a.len(), 800 * 480 * 4);
        assert_eq!(px(&a, 800, 2, 2), [0x10, 0x18, 0x20, 255]);
        assert!(a.chunks(4).any(|p| p[0] > 200 && p[1] > 120 && p[2] < 80), "amber text is drawn");
        r.set_vars(&[("velocity_kmh".into(), 7.0)], &[]);
        assert_eq!(r.text_of("speed").as_deref(), Some("7"));
        let b = r.poll_frame().unwrap();
        assert_ne!(a, b);
        assert!(r.poll_frame().is_none(), "no new frame without a change");
    }

    #[test]
    fn css_colours_and_sizes_are_painted() {
        let html = "<style>#a{width:10px;height:4px;background:#ff0000;margin:1px}.b{background:rgb(0,0,255);height:2px}</style><body style='margin:0'><div id=a></div><div class=b></div></body>";
        let mut r = EngineRenderer::new(20, 20, html);
        let f = r.poll_frame().unwrap();
        assert_eq!(px(&f, 20, 5, 2), [255, 0, 0, 255]);
        assert_eq!(px(&f, 20, 15, 2), [0, 0, 0, 0]);
        assert_eq!(px(&f, 20, 3, 7), [0, 0, 255, 255]);
    }

    #[test]
    fn scripts_write_variables_back() {
        let html = "<body><script>window.omsi = window.omsi || {}; window.omsi.update = function (d) { if (d.num.door > 0) omsi.setVar('lamp', 1); else window.omsi.setVar('lamp', 0); };</script></body>";
        let mut r = EngineRenderer::new(8, 8, html);
        r.set_vars(&[("door".into(), 1.0)], &[]);
        assert_eq!(r.take_events(), vec![("lamp".to_string(), 1.0)]);
        r.set_vars(&[("door".into(), 0.0)], &[]);
        assert_eq!(r.take_events(), vec![("lamp".to_string(), 0.0)]);
    }

    #[test]
    fn javascript_basics() {
        let html = "<body><p id=o></p><script>var t=0; for (var i=1;i<=4;i++){ if (i==3) continue; t+=i*2; } var f=(a,b)=>a>b?a:b; var o={x:[1,2,3]}; document.getElementById('o').textContent = t + ':' + f(3,9) + ':' + o.x.length + ':' + (7.126).toFixed(2) + ':' + 'ab'.padStart(4,'-') + ':' + Math.max(1,5,2) + ':' + typeof o;</script></body>";
        let r = EngineRenderer::new(8, 8, html);
        assert_eq!(r.text_of("o").as_deref(), Some("14:9:3:7.13:--ab:5:object"));
    }

    #[test]
    fn a_broken_script_does_not_stop_the_page() {
        let html = "<body style='background:#00ff00'><script>this is not javascript (</script></body>";
        let mut r = EngineRenderer::new(4, 4, html);
        let f = r.poll_frame().unwrap();
        assert_eq!(px(&f, 4, 1, 1), [0, 255, 0, 255]);
    }

    #[test]
    fn an_endless_loop_is_cut_off() {
        let html = "<body><script>while (true) {}</script></body>";
        let mut r = EngineRenderer::new(4, 4, html);
        assert!(r.poll_frame().is_some());
    }

    // ─────────────── operating the page ───────────────

    fn click(r: &mut EngineRenderer, x: f32, y: f32) {
        r.pointer(x, y, PointerKind::Down);
        r.pointer(x, y, PointerKind::Up);
    }

    const KEYPAD: &str = "<style>body{margin:0;width:200px;height:100px;background:#000}\
        button{width:40px;height:20px;margin:0;padding:0}</style><body>\
        <button id=a onclick=\"omsi.setVar('key', 1)\">A</button><button id=b>B</button><button id=c>C</button>\
        <script>document.getElementById('b').addEventListener('click', function (e) { omsi.setVar('key', 2); });\
        document.getElementById('c').onclick = () => omsi.setVar('key', 3);</script></body>";

    #[test]
    fn buttons_sit_in_a_row_and_take_clicks() {
        let mut r = EngineRenderer::new(200, 100, KEYPAD);
        // three 40px buttons side by side: x 0..40, 40..80, 80..120, all in the first row
        click(&mut r, 20.0, 10.0);
        assert_eq!(r.take_events(), vec![("key".to_string(), 1.0)]);
        click(&mut r, 60.0, 10.0);
        assert_eq!(r.take_events(), vec![("key".to_string(), 2.0)]);
        click(&mut r, 100.0, 10.0);
        assert_eq!(r.take_events(), vec![("key".to_string(), 3.0)]);
        // beside the buttons and below them nothing happens
        click(&mut r, 150.0, 10.0);
        click(&mut r, 20.0, 60.0);
        assert!(r.take_events().is_empty());
    }

    #[test]
    fn a_click_needs_a_press_and_a_release() {
        let mut r = EngineRenderer::new(200, 100, KEYPAD);
        r.pointer(20.0, 10.0, PointerKind::Up);
        assert!(r.take_events().is_empty(), "a release alone is no click");
        r.pointer(20.0, 10.0, PointerKind::Down);
        assert!(r.take_events().is_empty(), "a press alone is no click");
        r.pointer(20.0, 10.0, PointerKind::Up);
        assert_eq!(r.take_events().len(), 1);
    }

    #[test]
    fn inline_blocks_wrap_and_shrink_to_fit() {
        let html = "<style>body{margin:0;width:100px;height:100px}.k{display:inline-block;width:30px;height:10px;margin:0}</style>\
            <body><div class=k onclick=\"omsi.setVar('k',1)\"></div><div class=k onclick=\"omsi.setVar('k',2)\"></div>\
            <div class=k onclick=\"omsi.setVar('k',3)\"></div><div class=k onclick=\"omsi.setVar('k',4)\"></div></body>";
        let mut r = EngineRenderer::new(100, 100, html);
        // 3 fit into 100px, the fourth wraps to the second row
        click(&mut r, 75.0, 5.0);
        assert_eq!(r.take_events(), vec![("k".to_string(), 3.0)]);
        click(&mut r, 5.0, 15.0);
        assert_eq!(r.take_events(), vec![("k".to_string(), 4.0)]);
        // a button without a width is as wide as its label
        let plain = "<body style='margin:0'><button id=x onclick=\"omsi.setVar('x',1)\" style='padding:0'>Hi</button></body>";
        let mut r = EngineRenderer::new(200, 50, plain);
        click(&mut r, 190.0, 5.0);
        assert!(r.take_events().is_empty(), "the button does not fill the row");
        click(&mut r, 3.0, 5.0);
        assert_eq!(r.take_events().len(), 1);
    }

    #[test]
    fn clicks_bubble_and_can_be_stopped() {
        let html = "<body style='margin:0'><div id=outer onclick=\"omsi.setVar('outer',1)\" style='height:50px'>\
            <div id=inner style='height:20px'></div></div><script>\
            document.getElementById('inner').addEventListener('click', function (e) { omsi.setVar('inner', 1); if (e.x > 100) e.stopPropagation(); });</script></body>";
        let mut r = EngineRenderer::new(200, 100, html);
        click(&mut r, 10.0, 5.0);
        let ev = r.take_events();
        assert_eq!(ev, vec![("inner".to_string(), 1.0), ("outer".to_string(), 1.0)]);
        click(&mut r, 150.0, 5.0);
        assert_eq!(r.take_events(), vec![("inner".to_string(), 1.0)]);
        click(&mut r, 10.0, 40.0);
        assert_eq!(r.take_events(), vec![("outer".to_string(), 1.0)]);
    }

    #[test]
    fn a_span_in_text_is_hit_on_its_own() {
        let html = "<body style='margin:0;font-size:20px'>Go <span id=s onclick=\"omsi.setVar('s',1)\">here</span> now</body>";
        let mut r = EngineRenderer::new(300, 40, html);
        click(&mut r, 2.0, 10.0);
        assert!(r.take_events().is_empty());
        click(&mut r, 42.0, 10.0);
        assert_eq!(r.take_events(), vec![("s".to_string(), 1.0)]);
    }

    #[test]
    fn a_page_reacts_to_its_own_clicks() {
        // a route picker: the list is built by script, a click selects and writes the choice back
        let html = "<style>body{margin:0}.row{height:20px}.sel{background:#00ff00}</style><body><div id=list></div><script>\
            var routes = ['A', 'B', 'C']; var chosen = -1;\
            function draw() { var l = document.getElementById('list'); l.innerHTML = '';\
              routes.forEach(function (n, i) { var d = document.createElement('div'); d.className = 'row' + (i == chosen ? ' sel' : '');\
                d.textContent = n; d.addEventListener('click', function () { chosen = i; omsi.setVar('route', i); draw(); }); l.appendChild(d); }); }\
            draw();</script></body>";
        let mut r = EngineRenderer::new(50, 80, html);
        let f = r.poll_frame().unwrap();
        assert_eq!(px(&f, 50, 45, 30), [0, 0, 0, 0]);
        click(&mut r, 10.0, 30.0); // the second row
        assert_eq!(r.take_events(), vec![("route".to_string(), 1.0)]);
        let f = r.poll_frame().expect("the click changed the page");
        assert_eq!(px(&f, 50, 45, 30), [0, 255, 0, 255]);
        assert_eq!(px(&f, 50, 45, 10), [0, 0, 0, 0]);
    }

    #[test]
    fn class_list_and_inner_html() {
        let html = "<style>.on{background:#ff0000}</style><body style='margin:0'><div id=a style='height:4px'></div><div id=b></div><script>\
            var a = document.getElementById('a'); a.classList.add('x'); a.classList.toggle('on'); a.classList.toggle('x');\
            var t = a.classList.contains('on') + ',' + a.classList.contains('x');\
            document.getElementById('b').innerHTML = '<p id=p>hi <b>there</b></p>';\
            document.getElementById('b').setAttribute('id', 'b2'); window.t = t;</script></body>";
        let mut r = EngineRenderer::new(20, 20, html);
        let f = r.poll_frame().unwrap();
        assert_eq!(px(&f, 20, 5, 2), [255, 0, 0, 255]);
        assert_eq!(r.text_of("p").as_deref(), Some("hi there"));
    }

    #[test]
    fn timers_fire_when_due_and_can_be_cleared() {
        let html = "<body><p id=o></p><script>var n = 0; var once = 0; var id = setInterval(function () { n++; document.getElementById('o').textContent = 'n' + n;\
            if (n == 3) clearInterval(id); }, 100); setTimeout(function () { once++; omsi.setVar('once', once); }, 250);</script></body>";
        let mut r = EngineRenderer::new(8, 8, html);
        r.js.now = 0.05;
        assert!(!r.run_timers());
        r.js.now = 0.11;
        assert!(r.run_timers());
        assert_eq!(r.text_of("o").as_deref(), Some("n1"));
        r.js.now = 0.26;
        assert!(r.run_timers());
        assert_eq!(r.text_of("o").as_deref(), Some("n2"));
        assert_eq!(r.take_events(), vec![("once".to_string(), 1.0)]);
        r.js.now = 0.40;
        r.run_timers();
        assert_eq!(r.text_of("o").as_deref(), Some("n3"));
        r.js.now = 5.0;
        assert!(!r.run_timers(), "cleared, and the timeout only ran once");
    }

    #[test]
    fn arrays_objects_and_strings() {
        let html = "<body><p id=o></p><script>var a = [3, 1, 2]; var b = a.map(function (x) { return x * 2; }).filter(x => x > 2);\
            var o = {z: 1, a: 2}; var s = 'a-b-c'.split('-'); var sum = 0; a.forEach(function (x, i) { sum += x * i; });\
            document.getElementById('o').textContent = b.join('') + '|' + Object.keys(o).join('') + '|' + s.length + s[2] + '|' + sum + '|'\
              + a.indexOf(2) + a.includes(9) + '|' + a.slice(1).join('') + '|' + 'x.y'.replace('.', '+') + '|' + a.pop() + a.length;</script></body>";
        let r = EngineRenderer::new(8, 8, html);
        assert_eq!(r.text_of("o").as_deref(), Some("64|az|3c|5|2false|12|x+y|22"));
    }

    #[test]
    fn a_failing_callback_or_handler_does_not_stop_the_page() {
        let html = "<body style='margin:0'><div id=d style='height:10px' onclick=\"nope.x()\"></div><div id=e style='height:10px' onclick=\"omsi.setVar('ok', 1)\"></div>\
            <script>[1].forEach(function () { missing.call(); }); document.getElementById('d').addEventListener('click', function () { throw_it(); });</script></body>";
        let mut r = EngineRenderer::new(40, 40, html);
        click(&mut r, 5.0, 5.0);
        click(&mut r, 5.0, 15.0);
        assert_eq!(r.take_events(), vec![("ok".to_string(), 1.0)]);
    }

    #[test]
    fn a_click_can_press_a_trigger() {
        let html = "<body style='margin:0'><button style='padding:0;width:30px;height:10px' onclick=\"omsi.trigger('door_1'); omsi.setVar('seen', 1)\">Door</button></body>";
        let mut r = EngineRenderer::new(60, 20, html);
        click(&mut r, 5.0, 5.0);
        assert_eq!(r.take_triggers(), vec!["door_1".to_string()]);
        assert_eq!(r.take_events(), vec![("seen".to_string(), 1.0)]);
        assert!(r.take_triggers().is_empty());
    }

    #[test]
    fn the_controls_example_can_be_operated() {
        const CONTROLS: &str = include_str!("../../../docs/examples/htmltexture/controls.html");
        let mut r = EngineRenderer::new(800, 480, CONTROLS);
        r.set_vars(&[("velocity_kmh".into(), 34.2)], &[("next_stop".into(), "Hauptbahnhof".into())]);
        assert_eq!(r.text_of("speed").as_deref(), Some("34"));
        let idle = r.poll_frame().unwrap();
        click(&mut r, 55.0, 196.0); // 1
        click(&mut r, 55.0, 266.0); // 4
        assert_eq!(r.text_of("entry").as_deref(), Some("14"));
        assert!(r.take_events().is_empty(), "nothing is sent before OK");
        click(&mut r, 230.0, 406.0); // OK
        assert_eq!(r.take_events(), vec![("IBIS_Linie".to_string(), 14.0)]);
        click(&mut r, 500.0, 242.0); // Doors
        assert_eq!(r.take_triggers(), vec!["door_toggle".to_string()]);
        let typed = r.poll_frame().unwrap();
        assert_ne!(idle, typed, "the page shows what was pressed");
        // the flash of the pressed button ends by itself
        r.js.now += 1.0;
        assert!(r.run_timers());
        assert_eq!(px(&r.render(), 800, 420, 220), [0x2b, 0x3a, 0x4a, 255]);
    }
}