//! [`EngineRenderer`]: glues DOM, style, layout, paint and script together behind the
//! [`HtmlRenderer`] trait.

use super::*;

/// The engine as an [`HtmlRenderer`].
pub struct EngineRenderer {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) js: Interp,
    pub(crate) dirty: bool,
    pub(crate) warned: bool,
    pub(crate) start: std::time::Instant,
    /// A press is on the page and its release has not come yet (a click needs both).
    pub(crate) pressed: bool,
}

impl EngineRenderer {
    pub fn new(width: u32, height: u32, html: &str) -> EngineRenderer {
        let dom = Dom::parse(html);
        let scripts = dom.scripts.clone();
        log::debug!(
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
                Ok(()) => log::debug!("htmltexture: script {n} ran ({} steps)", js.steps),
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

    pub(crate) fn clock(&mut self) {
        self.js.now = self.start.elapsed().as_secs_f64();
    }

    /// Run the timers that are due. True when one ran (the page probably changed).
    pub(crate) fn run_timers(&mut self) -> bool {
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
    pub(crate) fn root_style(&self, lay: &Layouter) -> Style {
        let dom = &self.js.dom;
        match dom.nodes[dom.body].parent.filter(|&p| p != 0) {
            Some(h) => lay.style_of(h, &Style::default()),
            None => Style::default(),
        }
    }

    /// The element at a point of the texture (pixels); the body when nothing is there.
    pub(crate) fn hit_node(&self, x: f32, y: f32) -> usize {
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
    pub(crate) fn dispatch(&mut self, node: usize, ty: &str, x: f32, y: f32) {
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

    /// The `window.omsi` object.
    pub(crate) fn omsi(&self) -> Option<ObjRef> {
        match self.js.window.lock().unwrap().get("omsi") {
            Some(Val::Obj(o)) => Some(o.clone()),
            _ => None,
        }
    }

    /// Keep the latest value of every variable the host sends in `omsi.vars.num` and
    /// `omsi.vars.str`, under its lower-case name (and as sent, when that differs), so a page
    /// can read any of them at any time (`omsi.vars.num.engine_n`, `omsi.getVar("Door_1")`).
    pub(crate) fn store_vars(&mut self, num: &[(String, f32)], strs: &[(String, String)]) {
        let Some(omsi) = self.omsi() else { return };
        let vars = match omsi.lock().unwrap().get("vars") {
            Some(Val::Obj(o)) => o.clone(),
            _ => return,
        };
        let sub = |kind: &str| match vars.lock().unwrap().get(kind) {
            Some(Val::Obj(o)) => Some(o.clone()),
            _ => None,
        };
        if let Some(n) = sub("num") {
            let mut g = n.lock().unwrap();
            for (k, v) in num {
                let val = Val::Num(*v as f64);
                let lower = k.to_ascii_lowercase();
                if lower != *k {
                    g.insert(k.clone(), val.clone());
                }
                g.insert(lower, val);
            }
        }
        if let Some(s) = sub("str") {
            let mut g = s.lock().unwrap();
            for (k, v) in strs {
                let val = Val::Str(v.clone());
                let lower = k.to_ascii_lowercase();
                if lower != *k {
                    g.insert(k.clone(), val.clone());
                }
                g.insert(lower, val);
            }
        }
    }

    pub(crate) fn call_update(&mut self, num: &[(String, f32)], strs: &[(String, String)]) {
        let omsi = self.js.window.lock().unwrap().get("omsi").cloned();
        let Some(Val::Obj(omsi)) = omsi else {
            log::debug!("htmltexture: the page has no window.omsi");
            return;
        };
        let update = omsi.lock().unwrap().get("update").cloned();
        let Some(update @ Val::Func(_)) = update else {
            log::debug!("htmltexture: the page has no window.omsi.update function");
            return;
        };
        log::debug!("htmltexture: window.omsi.update with {} numeric and {} string variable(s)", num.len(), strs.len());
        let (veh, vars) = {
            let g = omsi.lock().unwrap();
            (g.get("vehicle").cloned().unwrap_or(Val::Undef), g.get("vars").cloned().unwrap_or(Val::Undef))
        };
        let nums: HashMap<String, Val> = num.iter().map(|(k, v)| (k.clone(), Val::Num(*v as f64))).collect();
        let strv: HashMap<String, Val> = strs.iter().map(|(k, v)| (k.clone(), Val::Str(v.clone()))).collect();
        let arg = obj_of(&[("num", Val::Obj(Arc::new(Mutex::new(nums)))), ("str", Val::Obj(Arc::new(Mutex::new(strv)))), ("vehicle", veh), ("vars", vars)]);
        self.js.steps = 0;
        let result = self.js.call(update, Val::Obj(omsi), vec![Val::Obj(arg)]);
        log::debug!("htmltexture: window.omsi.update took {} steps", self.js.steps);
        if let Err(e) = result {
            if !self.warned {
                log::warn!("htmltexture: omsi.update failed: {e}");
                self.warned = true;
            }
        }
    }

    pub(crate) fn render(&self) -> Vec<u8> {
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
        log::debug!("htmltexture: rendered {}x{} in {:?}", self.width, self.height, started.elapsed());
        cv.px
    }

    #[cfg(test)]
    pub(crate) fn text_of(&self, id: &str) -> Option<String> {
        self.js.dom.by_id(id).map(|i| self.js.dom.text_of(i))
    }
}

impl HtmlRenderer for EngineRenderer {
    fn set_vars(&mut self, num: &[(String, f32)], strs: &[(String, String)]) {
        self.clock();
        self.store_vars(num, strs);
        self.call_update(num, strs);
        self.dirty = true;
    }

    fn set_vehicle(&mut self, api: &crate::vehicle_api::ApiValue) {
        if let Some(omsi) = self.omsi() {
            omsi.lock().unwrap().insert("vehicle".to_string(), api_to_val(api));
        }
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
