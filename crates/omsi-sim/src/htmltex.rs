use crate::vehicle::VehicleInstance;
use std::collections::HashMap;
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
        None => Box::new(DummyRenderer::new(width, height)),
    }
}

pub struct DummyRenderer {
    width: u32,
    height: u32,
    frame: Option<Vec<u8>>,
}

impl DummyRenderer {
    pub fn new(width: u32, height: u32) -> DummyRenderer {
        DummyRenderer { width: width.max(1), height: height.max(1), frame: None }
    }
}

impl HtmlRenderer for DummyRenderer {
    fn set_vars(&mut self, num: &[(String, f32)], strs: &[(String, String)]) {
        let mut h: u32 = 2166136261;
        for (n, v) in num {
            for b in n.bytes().chain(v.to_bits().to_le_bytes()) {
                h = (h ^ b as u32).wrapping_mul(16777619);
            }
        }
        for (n, v) in strs {
            for b in n.bytes().chain(v.bytes()) {
                h = (h ^ b as u32).wrapping_mul(16777619);
            }
        }
        let px = [(h & 0xff) as u8 / 2 + 32, ((h >> 8) & 0xff) as u8 / 2 + 32, ((h >> 16) & 0xff) as u8 / 2 + 32, 255];
        let mut rgba = Vec::with_capacity((self.width * self.height * 4) as usize);
        for _ in 0..self.width * self.height {
            rgba.extend_from_slice(&px);
        }
        self.frame = Some(rgba);
    }

    fn poll_frame(&mut self) -> Option<Vec<u8>> {
        self.frame.take()
    }

    fn take_events(&mut self) -> Vec<(String, f32)> {
        Vec::new()
    }
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
                t.renderer.set_vars(&dn, &ds);
                for (n, v) in dn {
                    t.last_num.insert(n, v);
                }
                for (n, v) in ds {
                    t.last_str.insert(n, v);
                }
                t.started = true;
            }
            events.extend(t.renderer.take_events());
            if let Some(rgba) = t.renderer.poll_frame() {
                frames.push((t.script_index, t.width, t.height, rgba));
            }
        }
        for (name, v) in events {
            self.set_var(&name, v);
        }
        for (index, w, h, rgba) in frames {
            if let Some(st) = self.host.script_textures.get_mut(index) {
                if st.width == w && st.height == h && st.rgba.len() == rgba.len() {
                    st.rgba = rgba;
                    st.dirty = true;
                }
            }
        }
    }
}