//! The tree-walking interpreter: scopes, the built-in objects, DOM bindings, timers and
//! the step and depth limits that keep a broken page from hanging the game.

use super::*;

pub(crate) enum Flow {
    Next,
    Ret(Val),
    Brk,
    Cont,
}

pub(crate) struct Timer {
    pub(crate) id: u32,
    pub(crate) due: f64,
    pub(crate) every: Option<f64>,
    pub(crate) f: Val,
}

pub(crate) struct Interp {
    pub(crate) dom: Dom,
    pub(crate) global: Env,
    pub(crate) window: ObjRef,
    pub(crate) events: Vec<(String, f32)>,
    /// Triggers the page has pressed (`omsi.trigger(name)`).
    pub(crate) triggers: Vec<String>,
    pub(crate) steps: u32,
    pub(crate) depth: u32,
    /// Listeners by (element, event name): `onclick = f` and `addEventListener("click", f)`.
    pub(crate) handlers: HashMap<(usize, String), Vec<Val>>,
    pub(crate) timers: Vec<Timer>,
    pub(crate) next_timer: u32,
    /// Seconds since the page started; the renderer sets it before it runs anything.
    pub(crate) now: f64,
    /// An error inside a callback of a built-in method (`forEach` ...), raised by the caller.
    pub(crate) pending_err: Option<String>,
    /// Route, line and destination requests of the page (`omsi.setRoute(...)` ...).
    pub(crate) requests: Vec<crate::htmltex::HtmlRequest>,
}

impl Interp {
    pub(crate) fn new(dom: Dom) -> Interp {
        let global = Arc::new(Mutex::new(Scope { vars: HashMap::new(), parent: None }));
        let omsi = obj_of(&[
            ("setVar", Val::Nat(Nat::SetVar)),
            ("trigger", Val::Nat(Nat::Trigger)),
            ("getVar", Val::Nat(Nat::GetVar)),
            ("setRoute", Val::Nat(Nat::SetRoute)),
            ("setLine", Val::Nat(Nat::SetLine)),
            ("setDestination", Val::Nat(Nat::SetDestination)),
            (
                "depot",
                Val::Obj(obj_of(&[
                    ("name", Val::Str(String::new())),
                    ("lines", Val::Arr(Arc::new(Mutex::new(Vec::new())))),
                    ("routes", Val::Arr(Arc::new(Mutex::new(Vec::new())))),
                    ("destinations", Val::Arr(Arc::new(Mutex::new(Vec::new())))),
                ])),
            ),
            ("apiVersion", Val::Num(1.0)),
            ("vehicle", Val::Obj(obj_of(&[]))),
            ("vars", Val::Obj(obj_of(&[("num", Val::Obj(obj_of(&[]))), ("str", Val::Obj(obj_of(&[])))]))),
        ]);
        let window = obj_of(&[("omsi", Val::Obj(omsi))]);
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
            requests: Vec::new(),
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

    /// The latest value of a vehicle script variable by name, in any letter case: its number,
    /// else its text (`omsi.getVar(name)`). None until the host has sent it.
    pub(crate) fn omsi_var(&self, name: &str) -> Option<Val> {
        let lower = name.to_ascii_lowercase();
        let omsi = match self.window.lock().unwrap().get("omsi") {
            Some(Val::Obj(o)) => o.clone(),
            _ => return None,
        };
        let vars = match omsi.lock().unwrap().get("vars") {
            Some(Val::Obj(o)) => o.clone(),
            _ => return None,
        };
        for kind in ["num", "str"] {
            let map = match vars.lock().unwrap().get(kind) {
                Some(Val::Obj(o)) => o.clone(),
                _ => continue,
            };
            let found = map.lock().unwrap().get(&lower).cloned();
            if found.is_some() {
                return found;
            }
        }
        None
    }

    pub(crate) fn run(&mut self, src: &str) -> Result<(), String> {
        let prog = Parser { t: lex(src)?, i: 0 }.program()?;
        self.steps = 0;
        let env = self.global.clone();
        self.exec_block(&prog, &env).map(|_| ())
    }

    pub(crate) fn lookup(&self, env: &Env, name: &str) -> Option<Val> {
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

    pub(crate) fn assign_var(&self, env: &Env, name: &str, v: Val) {
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

    pub(crate) fn tick(&mut self) -> Result<(), String> {
        self.steps += 1;
        if self.steps > STEP_LIMIT {
            Err("script ran too long".into())
        } else {
            Ok(())
        }
    }

    pub(crate) fn exec_block(&mut self, stmts: &[Stmt], env: &Env) -> Result<Flow, String> {
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

    pub(crate) fn exec(&mut self, s: &Stmt, env: &Env) -> Result<Flow, String> {
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

    pub(crate) fn eval(&mut self, e: &Expr, env: &Env) -> Result<Val, String> {
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
                if !matches!(f, Val::Nat(_) | Val::Func(_)) {
                    return Err(format!("not a function: {}(...)", describe_expr(callee)));
                }
                self.call(f, this, argv)?
            }
        })
    }

    pub(crate) fn binary(&self, op: &str, l: Val, r: Val) -> Val {
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

    pub(crate) fn get_prop(&self, o: &Val, key: &str) -> Val {
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

    pub(crate) fn set_prop(&mut self, o: &Val, key: &str, v: Val) -> Result<(), String> {
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
    pub(crate) fn method(&mut self, o: &Val, name: &str, args: &[Val]) -> Option<Val> {
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

    pub(crate) fn call(&mut self, f: Val, _this: Val, args: Vec<Val>) -> Result<Val, String> {
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

    pub(crate) fn call_nat(&mut self, n: Nat, args: &[Val]) -> Val {
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
                        log::debug!("htmltexture: page calls setVar({name}, {v})");
                        self.events.push((name, v));
                    } else {
                        log::debug!("htmltexture: setVar({name}) ignored, the value is not a finite number");
                    }
                }
                Val::Undef
            }
            Nat::GetVar => {
                let name = args.first().map(to_str).unwrap_or_default();
                self.omsi_var(&name).unwrap_or(Val::Undef)
            }
            Nat::Trigger => {
                if let Some(name) = args.first().map(to_str).filter(|n| !n.is_empty()) {
                    log::debug!("htmltexture: page presses trigger {name}");
                    self.triggers.push(name);
                }
                Val::Undef
            }
            Nat::SetRoute | Nat::SetDestination => {
                let v = a(0);
                if v.is_finite() && v >= 0.0 {
                    let i = v as usize;
                    self.requests.push(if n == Nat::SetRoute { crate::htmltex::HtmlRequest::SetRoute(i) } else { crate::htmltex::HtmlRequest::SetDestination(i) });
                } else {
                    log::debug!("htmltexture: {n:?} ignored, the index is not a number");
                }
                Val::Undef
            }
            Nat::SetLine => {
                let line = args.first().map(to_str).unwrap_or_default();
                if !line.trim().is_empty() {
                    self.requests.push(crate::htmltex::HtmlRequest::SetLine(line));
                }
                Val::Undef
            }
            Nat::Log => {
                log::debug!("htmltexture console: {}", args.iter().map(to_str).collect::<Vec<_>>().join(" "));
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

/// Short source-like text of an expression, for error messages (`document.foo`, `a[...]`).
fn describe_expr(e: &Expr) -> String {
    match e {
        Expr::Ident(n) => n.clone(),
        Expr::Str(t) => format!("{t:?}"),
        Expr::Num(n) => fmt_num(*n),
        Expr::Member(o, k) => match k.as_ref() {
            Expr::Str(t) => format!("{}.{t}", describe_expr(o)),
            _ => format!("{}[..]", describe_expr(o)),
        },
        Expr::Call(c, _) => format!("{}(...)", describe_expr(c)),
        _ => "<expr>".into(),
    }
}