use crate::vehicle::VehicleInstance;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub trait HtmlRenderer: Send {
    fn set_vars(&mut self, num: &[(String, f32)], strs: &[(String, String)]);
    fn poll_frame(&mut self) -> Option<Vec<u8>>;
    fn take_events(&mut self) -> Vec<(String, f32)>;
}

pub type BackendFactory = fn(width: u32, height: u32, html: &str) -> Box<dyn HtmlRenderer>;

static BACKEND: OnceLock<BackendFactory> = OnceLock::new();

pub fn set_backend(factory: BackendFactory) -> bool {
    BACKEND.set(factory).is_ok()
}

fn make_renderer(width: u32, height: u32, html: &str) -> Box<dyn HtmlRenderer> {
    match BACKEND.get() {
        Some(f) => f(width, height, html),
        None => Box::new(crate::htmlengine::EngineRenderer::new(width, height, html)),
    }
}

/// Find `rel` (a `model.cfg` path, backslashes, any letter case) under the first of `dirs`
/// that has it: the model folder first, then the vehicle folder, then the model folder's
/// parent.
fn find_file(dirs: &[&Path], rel: &str) -> Option<PathBuf> {
    let mut all: Vec<PathBuf> = dirs.iter().map(|d| d.to_path_buf()).collect();
    if let Some(parent) = dirs.first().and_then(|d| d.parent()) {
        all.push(parent.to_path_buf());
    }
    all.iter().map(|d| omsi_cfg::resolve_path(d, rel)).find(|p| p.is_file())
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(p) = lower[from..].find(name) {
        let at = from + p;
        from = at + name.len();
        let before_ok = at == 0 || lower.as_bytes()[at - 1].is_ascii_whitespace();
        let rest = tag[from..].trim_start();
        if !before_ok || !rest.starts_with('=') {
            continue;
        }
        let rest = rest[1..].trim_start();
        return match rest.chars().next()? {
            q @ ('"' | '\'') => rest[1..].split(q).next().map(str::to_string),
            _ => rest.split(|c: char| c.is_whitespace() || c == '>').next().map(str::to_string),
        };
    }
    None
}

/// Read the page of an `[htmltexture]` and put its external style sheets and scripts
/// (`<link rel="stylesheet" href>`, `<script src>`) into it, so the engine sees one file.
/// A missing page gives an empty one (a blank texture, logged).
pub fn load_page(dirs: &[&Path], rel: &str) -> String {
    let Some(path) = find_file(dirs, rel) else {
        log::warn!("htmltexture: page {rel} not found");
        return String::new();
    };
    log::debug!("htmltexture: page {rel} -> {}", path.display());
    let Ok(bytes) = std::fs::read(&path) else {
        log::debug!("htmltexture: {} cannot be read", path.display());
        return String::new();
    };
    let html = String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}').to_string();
    let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut bases: Vec<&Path> = vec![base.as_path()];
    bases.extend(dirs.iter().copied());
    let read = |href: &str| -> Option<String> {
        let href = href.split(['?', '#']).next().unwrap_or(href);
        let text = find_file(&bases, href)
            .and_then(|p| std::fs::read(p).ok())
            .map(|b| String::from_utf8_lossy(&b).trim_start_matches('\u{feff}').to_string());
        match &text {
            Some(t) => log::debug!("htmltexture: inlined {href} ({} bytes)", t.len()),
            None => log::debug!("htmltexture: {href} not found, left as it is"),
        }
        text
    };
    let mut out = String::with_capacity(html.len());
    let mut rest = html.as_str();
    loop {
        let lower = rest.to_ascii_lowercase();
        let next = [lower.find("<link"), lower.find("<script")].into_iter().flatten().min();
        let Some(at) = next else { break };
        out.push_str(&rest[..at]);
        let Some(end) = rest[at..].find('>').map(|e| at + e + 1) else {
            rest = &rest[at..];
            break;
        };
        let tag = &rest[at..end];
        let is_link = lower[at..].starts_with("<link");
        if is_link {
            let sheet = attr(tag, "rel").map_or(false, |r| r.to_ascii_lowercase().contains("stylesheet"));
            match attr(tag, "href").filter(|_| sheet).and_then(|h| read(&h)) {
                Some(css) => out.push_str(&format!("<style>{css}</style>")),
                None => out.push_str(tag),
            }
            rest = &rest[end..];
        } else if let Some(js) = attr(tag, "src").and_then(|s| read(&s)) {
            out.push_str(&format!("<script>{js}</script>"));
            let after = &rest[end..];
            let close = after.to_ascii_lowercase().find("</script>").map(|c| c + "</script>".len()).unwrap_or(0);
            rest = &after[close..];
        } else {
            out.push_str(tag);
            rest = &rest[end..];
        }
    }
    out.push_str(rest);
    log::debug!("htmltexture: page {rel}: {} bytes after inlining", out.len());
    out
}

pub struct HtmlTexture {
    pub script_index: usize,
    pub width: u32,
    pub height: u32,
    renderer: Box<dyn HtmlRenderer>,
    last_num: HashMap<String, f32>,
    last_str: HashMap<String, String>,
    started: bool,
}

impl HtmlTexture {
    pub fn new(script_index: usize, width: i32, height: i32, html: &str) -> HtmlTexture {
        let (w, h) = (width.max(1) as u32, height.max(1) as u32);
        log::debug!("htmltexture #{script_index}: {w}x{h}, page of {} bytes", html.len());
        HtmlTexture {
            script_index,
            width: w,
            height: h,
            renderer: make_renderer(w, h, html),
            last_num: HashMap::new(),
            last_str: HashMap::new(),
            started: false,
        }
    }
}

impl VehicleInstance {
    pub fn update_html_textures(&mut self) {
        if self.html_textures.is_empty() {
            return;
        }
        let mut num = Vec::new();
        for (i, name) in self.ty.program.var_names.iter().enumerate() {
            num.push((name.clone(), self.state.vars[i]));
        }
        let mut strs = Vec::new();
        for (i, name) in self.ty.program.str_var_names.iter().enumerate() {
            strs.push((name.clone(), self.state.str_vars[i].clone()));
        }
        let mut events = Vec::new();
        let mut frames = Vec::new();
        for t in self.html_textures.iter_mut() {
            let dn: Vec<(String, f32)> = num
                .iter()
                .filter(|(n, v)| t.last_num.get(n) != Some(v))
                .cloned()
                .collect();
            let ds: Vec<(String, String)> = strs
                .iter()
                .filter(|(n, v)| t.last_str.get(n) != Some(v))
                .cloned()
                .collect();
            if !t.started || !dn.is_empty() || !ds.is_empty() {
                log::debug!(
                    "htmltexture #{}: {} numeric and {} string variable(s) to the page{}",
                    t.script_index,
                    dn.len(),
                    ds.len(),
                    if t.started { "" } else { " (first update)" }
                );
                t.renderer.set_vars(&dn, &ds);
                for (n, v) in dn {
                    t.last_num.insert(n, v);
                }
                for (n, v) in ds {
                    t.last_str.insert(n, v);
                }
                t.started = true;
            }
            let page_events = t.renderer.take_events();
            if !page_events.is_empty() {
                log::debug!("htmltexture #{}: the page sets {:?}", t.script_index, page_events);
            }
            events.extend(page_events);
            if let Some(rgba) = t.renderer.poll_frame() {
                log::debug!("htmltexture #{}: new frame of {} bytes", t.script_index, rgba.len());
                frames.push((t.script_index, t.width, t.height, rgba));
            }
        }
        for (name, v) in events {
            if !self.set_var(&name, v) {
                log::debug!("htmltexture: the page sets {name}, which the vehicle does not have");
            }
        }
        for (index, w, h, rgba) in frames {
            match self.host.script_textures.get_mut(index) {
                Some(st) if st.width == w && st.height == h && st.rgba.len() == rgba.len() => {
                    st.rgba = rgba;
                    st.dirty = true;
                }
                Some(st) => log::debug!(
                    "htmltexture #{index}: frame {w}x{h} does not fit the script texture {}x{}",
                    st.width,
                    st.height
                ),
                None => log::debug!("htmltexture #{index}: no script texture with this index"),
            }
        }
    }
}