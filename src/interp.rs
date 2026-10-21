//! Intérprete del lenguaje ACM.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::expr::{BinOp, Expr, Part, UnOp};
use crate::host::{self, Host};
use crate::out::Out;
use crate::parser::{self, Cmd, FuncDef, Kind, Stmt, TabArg};
use crate::value::{fmt_num, Value};
use crate::web::Web;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrKind {
    Runtime,
    Cancelled,
    Exit(i32),
}

#[derive(Debug, Clone)]
pub struct Error {
    pub msg: String,
    pub line: Option<usize>,
    pub kind: ErrKind,
}

impl Error {
    pub fn rt(msg: impl Into<String>) -> Self {
        Error { msg: msg.into(), line: None, kind: ErrKind::Runtime }
    }
    fn at(mut self, line: usize) -> Self {
        if self.line.is_none() {
            self.line = Some(line);
        }
        self
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::rt(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::rt(s)
    }
}

type R<T> = Result<T, Error>;

pub enum Flow {
    Next,
    Break,
    Continue,
    Return(Value),
}

#[derive(Clone)]
pub struct Options {
    pub dry_run: bool,
    pub continue_on_error: bool,
    pub failsafe: bool,
    pub dialogs: bool,
    /// Mostrar el resultado de las expresiones sueltas (REPL y -e).
    pub echo: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { dry_run: false, continue_on_error: false, failsafe: true, dialogs: true, echo: false }
    }
}

pub struct Interp {
    pub globals: HashMap<String, Value>,
    frames: Vec<HashMap<String, Value>>,
    funcs: HashMap<String, Rc<FuncDef>>,
    pub opts: Options,
    pub out: Out,
    host: Host,
    web: Web,
    type_delay: u64,
    cancel: Arc<AtomicBool>,
    rng: u64,
    pub base_dir: PathBuf,
    line: usize,
    import_depth: usize,
}

impl Interp {
    pub fn new(opts: Options, out: Out, cancel: Arc<AtomicBool>) -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15)
            | 1;
        let mut it = Interp {
            globals: HashMap::new(),
            frames: Vec::new(),
            funcs: HashMap::new(),
            opts,
            out,
            host: Host::new(),
            web: Web::new(),
            type_delay: 10,
            cancel,
            rng: seed,
            base_dir: std::env::current_dir().unwrap_or_default(),
            line: 0,
            import_depth: 0,
        };
        it.globals.insert("NL".into(), Value::str("\n"));
        it.globals.insert("TAB".into(), Value::str("\t"));
        it.globals.insert("OS".into(), Value::str(std::env::consts::OS));
        it.globals.insert("ARGS".into(), Value::List(vec![]));
        it
    }

    pub fn function_names(&self) -> Vec<String> {
        self.funcs.keys().cloned().collect()
    }

    // ---------------- Variables ----------------

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(f) = self.frames.last() {
            if let Some(v) = f.get(name) {
                return Some(v.clone());
            }
        }
        self.globals.get(name).cloned()
    }

    pub fn set_var(&mut self, name: &str, v: Value, declare: bool) {
        if let Some(f) = self.frames.last_mut() {
            if declare || f.contains_key(name) || !self.globals.contains_key(name) {
                f.insert(name.to_string(), v);
                return;
            }
        }
        self.globals.insert(name.to_string(), v);
    }

    // ---------------- Utilidades ----------------

    fn check_cancel(&self) -> R<()> {
        if self.cancel.load(Ordering::SeqCst) {
            Err(Error { msg: "ejecución cancelada".into(), line: None, kind: ErrKind::Cancelled })
        } else {
            Ok(())
        }
    }

    fn sleep_ms(&self, ms: u64) -> R<()> {
        let end = Instant::now() + Duration::from_millis(ms);
        loop {
            self.check_cancel()?;
            let now = Instant::now();
            if now >= end {
                return Ok(());
            }
            std::thread::sleep((end - now).min(Duration::from_millis(25)));
        }
    }

    /// En modo --dry-run registra la acción y devuelve true (no se ejecuta).
    fn dry(&self, desc: impl FnOnce() -> String) -> bool {
        if self.opts.dry_run {
            self.out.dry(self.line, &desc());
            true
        } else {
            false
        }
    }

    fn guard_input(&mut self) -> R<()> {
        if self.opts.failsafe {
            if let Ok((x, y)) = self.host.mouse_pos() {
                if x <= 0 && y <= 0 {
                    return Err(Error {
                        msg: "FAILSAFE: el ratón está en la esquina superior izquierda (0,0); script detenido. Usa FAILSAFE OFF o --no-failsafe para desactivarlo".into(),
                        line: None,
                        kind: ErrKind::Cancelled,
                    });
                }
            }
        }
        Ok(())
    }

    fn rand_u64(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x
    }

    fn rand_below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.rand_u64() % n as u64) as usize }
    }

    fn s(&mut self, e: &Expr) -> R<String> {
        Ok(self.eval(e)?.to_string())
    }

    fn n(&mut self, e: &Expr) -> R<f64> {
        let v = self.eval(e)?;
        v.as_num().ok_or_else(|| Error::rt(format!("se esperaba un número y se obtuvo '{}' ({})", v, v.type_name())))
    }

    fn i(&mut self, e: &Expr) -> R<i32> {
        Ok(self.n(e)?.round() as i32)
    }

    pub fn resolve_path(&self, p: &str) -> PathBuf {
        let path = Path::new(p);
        if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir().unwrap_or_default().join(path) }
    }

    // ---------------- Expresiones ----------------

    pub fn eval(&mut self, e: &Expr) -> R<Value> {
        match e {
            Expr::Lit(v) => Ok(v.clone()),
            Expr::Tpl(parts) => {
                let mut s = String::new();
                for p in parts {
                    match p {
                        Part::Text(t) => s.push_str(t),
                        Part::Expr(x, raw) => match self.eval(x) {
                            Ok(v) => s.push_str(&v.to_string()),
                            // `${NOMBRE}` con una variable inexistente se deja literal (p. ej. al generar
                            // código JavaScript con plantillas).
                            Err(_) if is_plain_ref(x) && !self.is_defined_ref(x) => {
                                s.push_str("${");
                                s.push_str(raw);
                                s.push('}');
                            }
                            Err(e) => return Err(e),
                        },
                    }
                }
                Ok(Value::Str(s))
            }
            Expr::Var(n) => self.get(n).ok_or_else(|| Error::rt(format!("variable no definida: {}", n))),
            Expr::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for it in items {
                    out.push(self.eval(it)?);
                }
                Ok(Value::List(out))
            }
            Expr::Index(b, i) => {
                let base = self.eval(b)?;
                let idx = self.eval(i)?;
                index_value(&base, &idx)
            }
            Expr::Member(b, name) => {
                let v = self.eval(b)?;
                match name.to_ascii_lowercase().as_str() {
                    "length" | "len" | "count" | "size" => Ok(Value::Num(length(&v) as f64)),
                    "first" => index_value(&v, &Value::Num(0.0)),
                    "last" => index_value(&v, &Value::Num(-1.0)),
                    other => Err(Error::rt(format!("propiedad desconocida '.{}'", other))),
                }
            }
            Expr::Call(name, args) => self.call(name, args),
            Expr::Unary(UnOp::Neg, x) => Ok(Value::Num(-self.n(x)?)),
            Expr::Unary(UnOp::Not, x) => Ok(Value::Bool(!self.eval(x)?.truthy())),
            Expr::Bin(op, a, b) => {
                let a = self.eval(a)?;
                let b = self.eval(b)?;
                binop(*op, &a, &b)
            }
            Expr::And(a, b) => Ok(Value::Bool(self.eval(a)?.truthy() && self.eval(b)?.truthy())),
            Expr::Or(a, b) => Ok(Value::Bool(self.eval(a)?.truthy() || self.eval(b)?.truthy())),
            Expr::Empty(x) => {
                let v = match &**x {
                    Expr::Var(n) => self.get(n).unwrap_or(Value::Null),
                    other => self.eval(other).unwrap_or(Value::Null),
                };
                Ok(Value::Bool(v.is_empty_like()))
            }
        }
    }

    fn call(&mut self, name: &str, arg_exprs: &[Expr]) -> R<Value> {
        let lname = name.to_lowercase();
        let mut args = Vec::with_capacity(arg_exprs.len());
        for a in arg_exprs {
            args.push(self.eval(a)?);
        }
        if let Some(def) = self.funcs.get(&lname).cloned() {
            return self.call_user(&def, args);
        }
        self.builtin(&lname, name, args)
    }

    fn call_user(&mut self, def: &FuncDef, args: Vec<Value>) -> R<Value> {
        if self.frames.len() > 200 {
            return Err(Error::rt("demasiada recursión (más de 200 llamadas anidadas)"));
        }
        if args.len() > def.params.len() {
            return Err(Error::rt(format!("{}() recibe {} argumentos y se pasaron {}", def.name, def.params.len(), args.len())));
        }
        let mut frame = HashMap::new();
        let mut it = args.into_iter();
        for p in &def.params {
            frame.insert(p.clone(), it.next().unwrap_or(Value::Null));
        }
        self.frames.push(frame);
        let r = self.exec_block(&def.body);
        self.frames.pop();
        match r? {
            Flow::Return(v) => Ok(v),
            _ => Ok(Value::Null),
        }
    }

    fn builtin(&mut self, lname: &str, name: &str, a: Vec<Value>) -> R<Value> {
        let argn = |n: usize| -> R<()> {
            if a.len() < n {
                Err(Error::rt(format!("{}() necesita {} argumento(s)", name, n)))
            } else {
                Ok(())
            }
        };
        let num = |v: &Value| -> R<f64> {
            v.as_num().ok_or_else(|| Error::rt(format!("{}(): '{}' no es un número", name, v)))
        };
        let sa = |i: usize| a.get(i).map(|v| v.to_string()).unwrap_or_default();
        Ok(match lname {
            // --- texto ---
            "len" | "length" => {
                argn(1)?;
                Value::Num(length(&a[0]) as f64)
            }
            "upper" => Value::Str(sa(0).to_uppercase()),
            "lower" => Value::Str(sa(0).to_lowercase()),
            "trim" => Value::Str(sa(0).trim().to_string()),
            "replace" => {
                argn(3)?;
                Value::Str(sa(0).replace(&sa(1), &sa(2)))
            }
            "split" => {
                argn(1)?;
                let sep = if a.len() > 1 { sa(1) } else { ",".into() };
                let s = sa(0);
                if sep.is_empty() {
                    Value::List(s.chars().map(|c| Value::Str(c.to_string())).collect())
                } else {
                    Value::List(s.split(sep.as_str()).map(|p| Value::str(p.trim())).collect())
                }
            }
            "lines" => Value::List(sa(0).lines().map(Value::str).collect()),
            "join" => {
                argn(1)?;
                let sep = if a.len() > 1 { sa(1) } else { ", ".into() };
                match &a[0] {
                    Value::List(l) => Value::Str(l.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(&sep)),
                    v => Value::Str(v.to_string()),
                }
            }
            "contains" | "includes" => {
                argn(2)?;
                Value::Bool(match &a[0] {
                    Value::List(l) => l.iter().any(|v| values_equal(v, &a[1])),
                    v => v.to_string().contains(&a[1].to_string()),
                })
            }
            "startswith" => Value::Bool(sa(0).starts_with(&sa(1))),
            "endswith" => Value::Bool(sa(0).ends_with(&sa(1))),
            "indexof" => {
                argn(2)?;
                Value::Num(match &a[0] {
                    Value::List(l) => l.iter().position(|v| values_equal(v, &a[1])).map(|p| p as f64).unwrap_or(-1.0),
                    v => {
                        let s = v.to_string();
                        s.find(&a[1].to_string()).map(|b| s[..b].chars().count() as f64).unwrap_or(-1.0)
                    }
                })
            }
            "substr" | "substring" => {
                argn(2)?;
                let chars: Vec<char> = sa(0).chars().collect();
                let start = (num(&a[1])?.max(0.0) as usize).min(chars.len());
                let len = if a.len() > 2 { num(&a[2])?.max(0.0) as usize } else { chars.len() - start };
                Value::Str(chars[start..(start + len).min(chars.len())].iter().collect())
            }
            "chr" => Value::Str(char::from_u32(num(a.first().unwrap_or(&Value::Null))? as u32).map(String::from).unwrap_or_default()),
            "ord" => Value::Num(sa(0).chars().next().map(|c| c as u32 as f64).unwrap_or(0.0)),
            "pad" | "padleft" => {
                argn(2)?;
                let s = sa(0);
                let width = num(&a[1])? as usize;
                let fill = if a.len() > 2 { sa(2).chars().next().unwrap_or(' ') } else { ' ' };
                let n = s.chars().count();
                Value::Str(if n >= width { s } else { std::iter::repeat_n(fill, width - n).collect::<String>() + &s })
            }
            // --- números y tipos ---
            "num" | "number" | "float" => {
                argn(1)?;
                Value::Num(num(&a[0])?)
            }
            "int" => {
                argn(1)?;
                Value::Num(num(&a[0])?.trunc())
            }
            "str" | "string" | "text" => Value::Str(sa(0)),
            "isnum" | "isnumber" => Value::Bool(a.first().and_then(|v| v.as_num()).is_some()),
            "type" | "typeof" => Value::str(a.first().map(|v| v.type_name()).unwrap_or("null")),
            "round" => {
                argn(1)?;
                let d = if a.len() > 1 { num(&a[1])? as i32 } else { 0 };
                let f = 10f64.powi(d);
                Value::Num((num(&a[0])? * f).round() / f)
            }
            "floor" => Value::Num(num(a.first().unwrap_or(&Value::Null))?.floor()),
            "ceil" => Value::Num(num(a.first().unwrap_or(&Value::Null))?.ceil()),
            "abs" => Value::Num(num(a.first().unwrap_or(&Value::Null))?.abs()),
            "min" | "max" => {
                let items: Vec<Value> = match a.as_slice() {
                    [Value::List(l)] => l.clone(),
                    _ => a.clone(),
                };
                let mut best: Option<f64> = None;
                for v in &items {
                    let x = num(v)?;
                    best = Some(match best {
                        None => x,
                        Some(b) => if lname == "min" { b.min(x) } else { b.max(x) },
                    });
                }
                best.map(Value::Num).unwrap_or(Value::Null)
            }
            "sum" => {
                let mut t = 0.0;
                if let Some(Value::List(l)) = a.first() {
                    for v in l {
                        t += num(v)?;
                    }
                }
                Value::Num(t)
            }
            "random" => match a.as_slice() {
                [] => Value::Num((self.rand_u64() % 1_000_000) as f64 / 1_000_000.0),
                [Value::List(l)] => {
                    let i = self.rand_below(l.len());
                    l.get(i).cloned().unwrap_or(Value::Null)
                }
                [Value::Num(lo), Value::Num(hi)] => {
                    let (lo, hi) = (*lo as i64, *hi as i64);
                    let span = (hi - lo + 1).max(1) as usize;
                    Value::Num((lo + self.rand_below(span) as i64) as f64)
                }
                [single] => single.clone(),
                many => {
                    let i = self.rand_below(many.len());
                    many[i].clone()
                }
            },
            "randint" => {
                argn(2)?;
                let (lo, hi) = (num(&a[0])? as i64, num(&a[1])? as i64);
                let span = (hi - lo + 1).max(1) as usize;
                Value::Num((lo + self.rand_below(span) as i64) as f64)
            }
            // --- listas ---
            "range" | "__range_incl" => {
                argn(1)?;
                let incl = lname == "__range_incl";
                let (start, end) = if a.len() == 1 { (0.0, num(&a[0])?) } else { (num(&a[0])?, num(&a[1])?) };
                let step = if a.len() > 2 { num(&a[2])? } else if end < start && incl { -1.0 } else { 1.0 };
                if step == 0.0 {
                    return Err(Error::rt("range(): el paso no puede ser 0"));
                }
                let mut out = Vec::new();
                let mut x = start;
                while (step > 0.0 && (x < end || (incl && x == end))) || (step < 0.0 && (x > end || (incl && x == end))) {
                    out.push(Value::Num(x));
                    x += step;
                    if out.len() > 10_000_000 {
                        return Err(Error::rt("range(): demasiados elementos"));
                    }
                }
                Value::List(out)
            }
            "push" | "append" => {
                argn(2)?;
                let mut l = match &a[0] {
                    Value::List(l) => l.clone(),
                    Value::Null => vec![],
                    v => vec![v.clone()],
                };
                l.extend(a[1..].iter().cloned());
                Value::List(l)
            }
            "slice" => {
                argn(2)?;
                let l = to_list(&a[0]);
                let n = l.len() as i64;
                let norm = |x: f64| -> usize {
                    let x = x as i64;
                    (if x < 0 { (n + x).max(0) } else { x.min(n) }) as usize
                };
                let s = norm(num(&a[1])?);
                let e = if a.len() > 2 { norm(num(&a[2])?) } else { n as usize };
                Value::List(if s < e { l[s..e].to_vec() } else { vec![] })
            }
            "reverse" => match a.first() {
                Some(Value::List(l)) => Value::List(l.iter().rev().cloned().collect()),
                Some(v) => Value::Str(v.to_string().chars().rev().collect()),
                None => Value::Null,
            },
            "sort" => {
                let mut l = to_list(a.first().unwrap_or(&Value::Null));
                l.sort_by(|x, y| match (x.as_num(), y.as_num()) {
                    (Some(p), Some(q)) => p.partial_cmp(&q).unwrap_or(std::cmp::Ordering::Equal),
                    _ => x.to_string().cmp(&y.to_string()),
                });
                Value::List(l)
            }
            "unique" => {
                let mut out: Vec<Value> = Vec::new();
                for v in to_list(a.first().unwrap_or(&Value::Null)) {
                    if !out.iter().any(|o| values_equal(o, &v)) {
                        out.push(v);
                    }
                }
                Value::List(out)
            }
            "json" | "tojson" => Value::Str(a.first().map(|v| v.to_json().to_string()).unwrap_or_default()),
            "parsejson" | "fromjson" => {
                let v: serde_json::Value = serde_json::from_str(&sa(0)).map_err(|e| Error::rt(format!("JSON inválido: {}", e)))?;
                Value::from_json(&v)
            }
            // --- sistema ---
            "env" => match std::env::var(sa(0)) {
                Ok(v) => Value::Str(v),
                Err(_) => a.get(1).cloned().unwrap_or(Value::Null),
            },
            "exists" => Value::Bool(self.resolve_path(&sa(0)).exists()),
            "read" | "readfile" => {
                let p = self.resolve_path(&sa(0));
                Value::Str(std::fs::read_to_string(&p).map_err(|e| Error::rt(format!("no se pudo leer '{}': {}", p.display(), e)))?)
            }
            "readlines" => {
                let p = self.resolve_path(&sa(0));
                let s = std::fs::read_to_string(&p).map_err(|e| Error::rt(format!("no se pudo leer '{}': {}", p.display(), e)))?;
                Value::List(s.lines().map(Value::str).collect())
            }
            "listdir" | "files" => {
                let dir = if a.is_empty() { ".".to_string() } else { sa(0) };
                let p = self.resolve_path(&dir);
                let mut names: Vec<String> = std::fs::read_dir(&p)
                    .map_err(|e| Error::rt(format!("no se pudo listar '{}': {}", p.display(), e)))?
                    .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
                    .collect();
                names.sort();
                Value::List(names.into_iter().map(Value::Str).collect())
            }
            "exec" | "shell" | "run" => {
                let cmd = sa(0);
                if self.dry(|| format!("EXEC {}", cmd)) {
                    return Ok(Value::str(""));
                }
                let r = host::exec(&cmd)?;
                self.globals.insert("EXITCODE".into(), Value::Num(r.code as f64));
                Value::Str(r.stdout.trim_end().to_string())
            }
            "now" | "date" => {
                use std::fmt::Write as _;
                let fmt = if a.is_empty() { "%Y-%m-%d %H:%M:%S".to_string() } else { sa(0) };
                let mut s = String::new();
                write!(s, "{}", chrono::Local::now().format(&fmt)).map_err(|_| Error::rt(format!("formato de fecha inválido: {}", fmt)))?;
                Value::Str(s)
            }
            "time" | "millis" => Value::Num(
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0),
            ),
            "clipboard" | "getclipboard" => {
                if self.opts.dry_run {
                    return Ok(Value::str(""));
                }
                Value::Str(self.host.clip_get()?)
            }
            "mousex" | "mousey" | "mousepos" => {
                if self.opts.dry_run {
                    return Ok(Value::Num(0.0));
                }
                let (x, y) = self.host.mouse_pos()?;
                match lname {
                    "mousex" => Value::Num(x as f64),
                    "mousey" => Value::Num(y as f64),
                    _ => Value::List(vec![Value::Num(x as f64), Value::Num(y as f64)]),
                }
            }
            "screenwidth" | "screenheight" => {
                if self.opts.dry_run {
                    return Ok(Value::Num(0.0));
                }
                let (w, h) = self.host.screen_size()?;
                Value::Num(if lname == "screenwidth" { w } else { h } as f64)
            }
            "pixel" => {
                argn(2)?;
                if self.opts.dry_run {
                    return Ok(Value::str("#000000"));
                }
                Value::Str(host::pixel(num(&a[0])? as i32, num(&a[1])? as i32)?)
            }
            "windows" => Value::List(host::windows()?.into_iter().map(|w| Value::Str(w.title)).collect()),
            "activewindow" => Value::Str(host::windows()?.into_iter().find(|w| w.focused).map(|w| w.title).unwrap_or_default()),
            "windowexists" => Value::Bool(host::find_window(&sa(0))?.is_some()),
            // --- web ---
            "webtext" => Value::Str(self.web.get_text(&sa(0))?),
            "webexists" => Value::Bool(self.web.exists(&sa(0))?),
            "webattr" => {
                argn(2)?;
                self.web.attr(&sa(0), &sa(1))?
            }
            "weburl" => Value::Str(self.web.url()?),
            "webtitle" => Value::Str(self.web.title()?),
            "webjs" => self.web.js(&sa(0))?,
            "webtabs" => Value::Num(self.web.tab_count() as f64),
            _ => return Err(Error::rt(format!("función desconocida: {}()", name))),
        })
    }

    // ---------------- Sentencias ----------------

    pub fn exec_block(&mut self, stmts: &[Stmt]) -> R<Flow> {
        for s in stmts {
            match self.exec_repeat(s) {
                Ok(Flow::Next) => {}
                Ok(f) => return Ok(f),
                Err(err) => {
                    let err = err.at(s.line);
                    if err.kind == ErrKind::Runtime && self.opts.continue_on_error {
                        self.out.error(err.line, &err.msg);
                        continue;
                    }
                    return Err(err);
                }
            }
        }
        Ok(Flow::Next)
    }

    fn exec_repeat(&mut self, s: &Stmt) -> R<Flow> {
        self.check_cancel()?;
        self.line = s.line;
        if !matches!(s.kind, Kind::Func(_)) {
            self.out.step(s.line, &s.src);
        }
        for i in 0..s.repeat {
            if i > 0 {
                self.sleep_ms(50)?;
            }
            match self.exec_stmt(s)? {
                Flow::Next => {}
                f => return Ok(f),
            }
        }
        Ok(Flow::Next)
    }

    fn exec_stmt(&mut self, s: &Stmt) -> R<Flow> {
        match &s.kind {
            Kind::Var { name, value } => {
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Null,
                };
                self.set_var(name, v, true);
            }
            Kind::Assign { target, op, value } => {
                let rhs = self.eval(value)?;
                match &target.index {
                    None => {
                        let v = match op {
                            None => rhs,
                            Some(op) => {
                                let cur = self.get(&target.name).ok_or_else(|| Error::rt(format!("variable no definida: {}", target.name)))?;
                                binop(*op, &cur, &rhs)?
                            }
                        };
                        self.set_var(&target.name, v, false);
                    }
                    Some(ie) => {
                        let idx = self.eval(ie)?;
                        let mut list = match self.get(&target.name) {
                            Some(Value::List(l)) => l,
                            Some(other) => return Err(Error::rt(format!("{} no es una lista ({})", target.name, other.type_name()))),
                            None => return Err(Error::rt(format!("variable no definida: {}", target.name))),
                        };
                        let i = list_index(list.len(), &idx, true)?;
                        if i >= list.len() {
                            list.resize(i + 1, Value::Null);
                        }
                        list[i] = match op {
                            None => rhs,
                            Some(op) => binop(*op, &list[i], &rhs)?,
                        };
                        self.set_var(&target.name, Value::List(list), false);
                    }
                }
            }
            Kind::Expr(e) => {
                let v = self.eval(e)?;
                if self.opts.echo && v != Value::Null {
                    let shown = match &v {
                        Value::Str(_) => v.to_string(),
                        _ => v.to_json().to_string(),
                    };
                    self.out.print(&shown);
                }
            }
            Kind::If { branches, otherwise } => {
                for (cond, body) in branches {
                    if self.eval(cond)?.truthy() {
                        return self.exec_block(body);
                    }
                }
                return self.exec_block(otherwise);
            }
            Kind::ForIn { var, iter, body } => {
                let items = match self.eval(iter)? {
                    Value::List(l) => l,
                    Value::Num(n) => (0..n.max(0.0) as i64).map(|i| Value::Num(i as f64)).collect(),
                    Value::Null => vec![],
                    Value::Str(s) if s.contains('\n') => s.lines().map(Value::str).collect(),
                    Value::Str(s) if s.contains(',') => s.split(',').map(|p| Value::str(p.trim())).collect(),
                    Value::Str(s) if s.is_empty() => vec![],
                    other => vec![other],
                };
                for item in items {
                    self.check_cancel()?;
                    self.set_var(var, item, true);
                    match self.exec_block(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
            }
            Kind::ForC { init, cond, step, body } => {
                if let Some(i) = init {
                    self.exec_stmt(i)?;
                }
                loop {
                    self.check_cancel()?;
                    if let Some(c) = cond {
                        if !self.eval(c)?.truthy() {
                            break;
                        }
                    }
                    match self.exec_block(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                    if let Some(st) = step {
                        self.exec_stmt(st)?;
                    }
                }
            }
            Kind::While { cond, body } => loop {
                self.check_cancel()?;
                if !self.eval(cond)?.truthy() {
                    break;
                }
                match self.exec_block(body)? {
                    Flow::Break => break,
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    _ => {}
                }
            },
            Kind::Loop { count, body } => {
                let n = match count {
                    Some(e) => Some(self.n(e)?.max(0.0) as u64),
                    None => None,
                };
                let mut i: u64 = 0;
                loop {
                    if let Some(n) = n {
                        if i >= n {
                            break;
                        }
                    }
                    self.check_cancel()?;
                    self.set_var("LOOP_INDEX", Value::Num(i as f64), false);
                    match self.exec_block(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                    i += 1;
                }
            }
            Kind::Break => return Ok(Flow::Break),
            Kind::Continue => return Ok(Flow::Continue),
            Kind::Return(e) => {
                let v = match e {
                    Some(e) => self.eval(e)?,
                    None => Value::Null,
                };
                return Ok(Flow::Return(v));
            }
            Kind::Exit(e) => {
                let code = match e {
                    Some(e) => self.n(e)? as i32,
                    None => 0,
                };
                return Err(Error { msg: format!("EXIT {}", code), line: None, kind: ErrKind::Exit(code) });
            }
            Kind::Func(def) => {
                self.funcs.insert(def.name.to_lowercase(), def.clone());
            }
            Kind::Import(e) => {
                let p = self.s(e)?;
                let path = {
                    let pp = Path::new(&p);
                    if pp.is_absolute() { pp.to_path_buf() } else { self.base_dir.join(pp) }
                };
                if self.import_depth > 20 {
                    return Err(Error::rt("IMPORT anidado demasiadas veces"));
                }
                let src = std::fs::read_to_string(&path).map_err(|e| Error::rt(format!("no se pudo importar '{}': {}", path.display(), e)))?;
                let prog = parser::parse_program(&src)
                    .map_err(|pe| Error::rt(format!("error de sintaxis en '{}' (línea {}): {}", path.display(), pe.line, pe.msg)))?;
                self.import_depth += 1;
                let r = self.exec_block(&prog);
                self.import_depth -= 1;
                r.map_err(|mut e| {
                    if e.kind == ErrKind::Runtime {
                        e.msg = format!("{} (en {}:{})", e.msg, path.display(), e.line.unwrap_or(0));
                        e.line = None;
                    }
                    e
                })?;
            }
            Kind::Cmd(c) => self.exec_cmd(c)?,
        }
        Ok(Flow::Next)
    }

    fn type_text(&mut self, text: &str) -> R<()> {
        for ch in text.chars() {
            self.check_cancel()?;
            self.host.type_char(ch)?;
            if self.type_delay > 0 {
                std::thread::sleep(Duration::from_millis(self.type_delay));
            }
        }
        Ok(())
    }

    fn parse_keys(&self, combo: &str) -> R<Vec<enigo::Key>> {
        let t = combo.trim();
        if t == "+" {
            return Ok(vec![enigo::Key::Unicode('+')]);
        }
        let mut keys = Vec::new();
        for tok in t.split('+').map(str::trim).filter(|s| !s.is_empty()) {
            let k = host::key_from_name(tok)
                .or_else(|| self.get(tok).and_then(|v| host::key_from_name(&v.to_string())))
                .ok_or_else(|| Error::rt(format!("tecla desconocida: '{}'", tok)))?;
            keys.push(k);
        }
        if keys.is_empty() {
            return Err(Error::rt("no se indicó ninguna tecla"));
        }
        Ok(keys)
    }

    fn exec_cmd(&mut self, c: &Cmd) -> R<()> {
        match c {
            Cmd::Click { button, count, pos, write } => {
                let p = match pos {
                    Some((x, y)) => Some((self.i(x)?, self.i(y)?)),
                    None => None,
                };
                let text = match write {
                    Some(w) => Some(self.s(w)?),
                    None => None,
                };
                let desc = || {
                    let name = match (button, count) {
                        (_, 2) => "DOUBLECLICK",
                        (parser::Btn::Right, _) => "RIGHTCLICK",
                        (parser::Btn::Middle, _) => "MIDDLECLICK",
                        _ => "CLICK",
                    };
                    let mut d = match p {
                        Some((x, y)) => format!("{} {} {}", name, x, y),
                        None => name.to_string(),
                    };
                    if let Some(t) = &text {
                        d.push_str(&format!(" WRITE {:?}", t));
                    }
                    d
                };
                if self.dry(desc) {
                    return Ok(());
                }
                self.guard_input()?;
                self.host.click(*button, *count, p)?;
                if let Some(t) = text {
                    self.sleep_ms(100)?;
                    self.type_text(&t)?;
                }
            }
            Cmd::Move(x, y) => {
                let (x, y) = (self.i(x)?, self.i(y)?);
                if self.dry(|| format!("MOVE {} {}", x, y)) {
                    return Ok(());
                }
                self.guard_input()?;
                self.host.move_to(x, y)?;
            }
            Cmd::Drag(a, b, c2, d) => {
                let (x1, y1, x2, y2) = (self.i(a)?, self.i(b)?, self.i(c2)?, self.i(d)?);
                if self.dry(|| format!("DRAG {} {} TO {} {}", x1, y1, x2, y2)) {
                    return Ok(());
                }
                self.guard_input()?;
                self.host.drag(x1, y1, x2, y2)?;
            }
            Cmd::Scroll { amount, horizontal } => {
                let n = self.i(amount)?;
                if self.dry(|| format!("SCROLL {}{}", n, if *horizontal { " HORIZONTAL" } else { "" })) {
                    return Ok(());
                }
                self.guard_input()?;
                self.host.scroll(n, *horizontal)?;
            }
            Cmd::MouseButton { button, down } => {
                if self.dry(|| format!("{} {:?}", if *down { "MOUSEDOWN" } else { "MOUSEUP" }, button)) {
                    return Ok(());
                }
                self.guard_input()?;
                self.host.mouse_button(*button, *down)?;
            }
            Cmd::Write(e) => {
                let t = self.s(e)?;
                if self.dry(|| format!("WRITE {:?}", t)) {
                    return Ok(());
                }
                self.guard_input()?;
                self.type_text(&t)?;
            }
            Cmd::Paste(e) => {
                let t = self.s(e)?;
                if self.dry(|| format!("PASTE {:?}", t)) {
                    return Ok(());
                }
                self.guard_input()?;
                self.host.paste(&t)?;
            }
            Cmd::Keys { combo, mode } => {
                let text = self.s(combo)?;
                let keys = self.parse_keys(&text)?;
                if self.dry(|| format!("KEY {} ({:?})", text, mode)) {
                    return Ok(());
                }
                self.guard_input()?;
                self.host.keys(&keys, *mode)?;
            }
            Cmd::Delay { ms, mult } => {
                let total = (self.n(ms)? * mult).max(0.0) as u64;
                if self.dry(|| format!("DELAY {}ms", total)) {
                    return Ok(());
                }
                self.sleep_ms(total)?;
            }
            Cmd::TypeDelay(e) => self.type_delay = self.n(e)?.max(0.0) as u64,
            Cmd::Print(items) => {
                let mut parts = Vec::new();
                for it in items {
                    parts.push(self.s(it)?);
                }
                self.out.print(&parts.join(" "));
            }
            Cmd::Assert(cond, msg) => {
                if !self.eval(cond)?.truthy() {
                    let m = match msg {
                        Some(m) => self.s(m)?,
                        None => "la condición es falsa".into(),
                    };
                    // En simulación los datos leídos (portapapeles, web...) son ficticios.
                    if self.dry(|| format!("ASSERT falló (se ignora en --dry-run): {}", m)) {
                        return Ok(());
                    }
                    return Err(Error::rt(format!("ASSERT falló: {}", m)));
                }
            }
            Cmd::Ask { var, prompt } => {
                let p = match prompt {
                    Some(p) => self.s(p)?,
                    None => format!("{}: ", var),
                };
                if self.out.json {
                    self.out.event(serde_json::json!({"event": "ask", "var": var, "prompt": p}));
                } else {
                    eprint!("{} ", p.trim_end());
                    let _ = std::io::stderr().flush();
                }
                let mut line = String::new();
                std::io::stdin().lock().read_line(&mut line).map_err(|e| Error::rt(e.to_string()))?;
                self.set_var(var, Value::Str(line.trim_end_matches(['\r', '\n']).to_string()), false);
            }
            Cmd::Open(e) => {
                let t = self.s(e)?;
                if self.dry(|| format!("OPEN {:?}", t)) {
                    return Ok(());
                }
                host::open(&t)?;
            }
            Cmd::Exec { cmd, into } => {
                let line = self.s(cmd)?;
                if self.dry(|| format!("EXEC {:?}", line)) {
                    if let Some(v) = into {
                        self.set_var(v, Value::str(""), false);
                    }
                    return Ok(());
                }
                let r = host::exec(&line)?;
                self.globals.insert("EXITCODE".into(), Value::Num(r.code as f64));
                match into {
                    Some(v) => self.set_var(v, Value::Str(r.stdout.trim_end().to_string()), false),
                    None => {
                        let o = r.stdout.trim_end();
                        if !o.is_empty() {
                            self.out.print(o);
                        }
                        let er = r.stderr.trim_end();
                        if !er.is_empty() {
                            self.out.warn(er);
                        }
                    }
                }
            }
            Cmd::Alert(e) => {
                let t = self.s(e)?;
                self.out.event(serde_json::json!({"event": "alert", "message": t}));
                if self.dry(|| format!("ALERT {:?}", t)) {
                    return Ok(());
                }
                if self.opts.dialogs {
                    host::alert(&t)?;
                } else if !self.out.json {
                    self.out.print(&format!("[ALERT] {}", t));
                }
            }
            Cmd::Notify(m, title) => {
                let t = self.s(m)?;
                let ti = match title {
                    Some(x) => self.s(x)?,
                    None => "acm".into(),
                };
                self.out.event(serde_json::json!({"event": "notify", "message": t, "title": ti}));
                if self.dry(|| format!("NOTIFY {:?}", t)) {
                    return Ok(());
                }
                if self.opts.dialogs {
                    if let Err(e) = host::notify(&t, &ti) {
                        self.out.warn(&e);
                    }
                } else if !self.out.json {
                    self.out.print(&format!("[NOTIFY] {}", t));
                }
            }
            Cmd::WriteFile { path, content, append } => {
                let p = self.s(path)?;
                let body = self.s(content)?;
                if self.dry(|| format!("{} {:?} ({} bytes)", if *append { "APPENDFILE" } else { "WRITEFILE" }, p, body.len())) {
                    return Ok(());
                }
                let full = self.resolve_path(&p);
                host::ensure_parent(&full.to_string_lossy())?;
                let r = if *append {
                    std::fs::OpenOptions::new().create(true).append(true).open(&full).and_then(|mut f| f.write_all(body.as_bytes()))
                } else {
                    std::fs::write(&full, body)
                };
                r.map_err(|e| Error::rt(format!("no se pudo escribir '{}': {}", full.display(), e)))?;
            }
            Cmd::DeleteFile(e) => {
                let p = self.s(e)?;
                if self.dry(|| format!("DELETEFILE {:?}", p)) {
                    return Ok(());
                }
                let full = self.resolve_path(&p);
                if full.exists() {
                    std::fs::remove_file(&full).map_err(|e| Error::rt(format!("no se pudo borrar '{}': {}", full.display(), e)))?;
                }
            }
            Cmd::MkDir(e) => {
                let p = self.s(e)?;
                if self.dry(|| format!("MKDIR {:?}", p)) {
                    return Ok(());
                }
                std::fs::create_dir_all(self.resolve_path(&p)).map_err(|e| Error::rt(e.to_string()))?;
            }
            Cmd::ClipSet(e) => {
                let t = self.s(e)?;
                if self.dry(|| format!("CLIPSET {:?}", t)) {
                    return Ok(());
                }
                self.host.clip_set(&t)?;
            }
            Cmd::Focus(e) => {
                let t = self.s(e)?;
                if self.dry(|| format!("FOCUS {:?}", t)) {
                    return Ok(());
                }
                host::focus_window(&t)?;
                self.sleep_ms(150)?;
            }
            Cmd::WaitWindow(q, timeout) => {
                let t = self.s(q)?;
                let ms = match timeout {
                    Some(x) => self.n(x)? as u64,
                    None => 10_000,
                };
                if self.dry(|| format!("WAITWINDOW {:?} {}ms", t, ms)) {
                    return Ok(());
                }
                let start = Instant::now();
                while host::find_window(&t)?.is_none() {
                    if start.elapsed() >= Duration::from_millis(ms) {
                        return Err(Error::rt(format!("tiempo agotado esperando la ventana '{}'", t)));
                    }
                    self.sleep_ms(200)?;
                }
            }
            Cmd::Screenshot { path, region } => {
                let p = self.s(path)?;
                let reg = match region {
                    Some([x, y, w, h]) => Some((self.i(x)?, self.i(y)?, self.i(w)?.max(1) as u32, self.i(h)?.max(1) as u32)),
                    None => None,
                };
                if self.dry(|| format!("SCREENSHOT {:?}", p)) {
                    return Ok(());
                }
                let full = self.resolve_path(&p);
                host::screenshot(&full.to_string_lossy(), reg)?;
                self.out.event(serde_json::json!({"event": "screenshot", "path": full.to_string_lossy()}));
            }
            Cmd::OnError(cont) => self.opts.continue_on_error = *cont,
            Cmd::Failsafe(on) => self.opts.failsafe = *on,
            // ---------------- Web ----------------
            Cmd::WebBrowser(e) => {
                let spec = self.s(e)?;
                self.web.configure(&spec)?;
            }
            Cmd::WebProfile(e) => {
                let p = self.s(e)?;
                let full = if p.trim().is_empty() { String::new() } else { self.resolve_path(&p).to_string_lossy().to_string() };
                self.web.set_profile(&full);
            }
            Cmd::WebTimeout(e) => self.web.timeout = Duration::from_millis(self.n(e)?.max(0.0) as u64),
            Cmd::WebOpen { url, nowait } => {
                let u = self.s(url)?;
                if self.dry(|| format!("WEBOPEN {:?}{}", u, if *nowait { " (sin esperar)" } else { "" })) {
                    return Ok(());
                }
                self.web.open(&u, *nowait)?;
            }
            Cmd::WebTab { arg, nowait } => match arg {
                TabArg::New => {
                    if self.dry(|| "WEBTAB NEW".into()) {
                        return Ok(());
                    }
                    self.web.new_tab(None, false)?;
                }
                TabArg::Close => {
                    if self.dry(|| "WEBTAB CLOSE".into()) {
                        return Ok(());
                    }
                    self.web.close_tab()?;
                }
                TabArg::Value(e) => {
                    let v = self.eval(e)?;
                    let index = match &v {
                        Value::Num(n) => Some(*n),
                        Value::Str(s) => s.trim().parse::<f64>().ok().filter(|_| !s.contains('.')),
                        _ => None,
                    };
                    match index {
                        Some(i) => {
                            if self.dry(|| format!("WEBTAB {}", fmt_num(i))) {
                                return Ok(());
                            }
                            self.web.switch_tab(i.max(0.0) as usize)?;
                        }
                        None => {
                            let u = v.to_string();
                            if self.dry(|| format!("WEBTAB {:?}{}", u, if *nowait { " (sin esperar)" } else { "" })) {
                                return Ok(());
                            }
                            self.web.new_tab(Some(&u), *nowait)?;
                        }
                    }
                }
            },
            Cmd::WebClick(e) => {
                let sel = self.s(e)?;
                if self.dry(|| format!("WEBCLICK {:?}", sel)) {
                    return Ok(());
                }
                self.web.click(&sel)?;
            }
            Cmd::WebInput(s, t) => {
                let sel = self.s(s)?;
                let text = self.s(t)?;
                if self.dry(|| format!("WEBINPUT {:?} {:?}", sel, text)) {
                    return Ok(());
                }
                self.web.input(&sel, &text)?;
            }
            Cmd::WebGet(s, var) => {
                let sel = self.s(s)?;
                if self.dry(|| format!("WEBGET {:?} -> {}", sel, var)) {
                    self.set_var(var, Value::str(""), false);
                    return Ok(());
                }
                let t = self.web.get_text(&sel)?;
                self.set_var(var, Value::Str(t), false);
            }
            Cmd::WebWait { sel, cond, timeout } => {
                let s = self.s(sel)?;
                let t = match timeout {
                    Some(x) => Some(Duration::from_millis(self.n(x)? as u64)),
                    None => None,
                };
                if self.dry(|| format!("WEBWAIT {:?} {}", s, cond)) {
                    return Ok(());
                }
                self.web.wait(&s, cond, t)?;
            }
            Cmd::WebSelect(s, t) => {
                let sel = self.s(s)?;
                let opt = self.s(t)?;
                if self.dry(|| format!("WEBSELECT {:?} {:?}", sel, opt)) {
                    return Ok(());
                }
                self.web.select(&sel, &opt)?;
            }
            Cmd::WebCheck(s, v) => {
                let sel = self.s(s)?;
                let want = self.eval(v)?.truthy();
                if self.dry(|| format!("WEBCHECK {:?} {}", sel, want)) {
                    return Ok(());
                }
                self.web.check(&sel, want)?;
            }
            Cmd::WebScroll(e) => {
                let sel = self.s(e)?;
                if self.dry(|| format!("WEBSCROLL {:?}", sel)) {
                    return Ok(());
                }
                self.web.scroll_to(&sel)?;
            }
            Cmd::WebScreenshot(e) => {
                let p = self.s(e)?;
                if self.dry(|| format!("WEBSCREENSHOT {:?}", p)) {
                    return Ok(());
                }
                let full = self.resolve_path(&p);
                self.web.screenshot(&full.to_string_lossy())?;
                self.out.event(serde_json::json!({"event": "screenshot", "path": full.to_string_lossy()}));
            }
            Cmd::WebJs { code, into } => {
                let js = self.s(code)?;
                if self.dry(|| format!("WEBJS {:?}", js)) {
                    return Ok(());
                }
                let v = self.web.js(&js)?;
                match into {
                    Some(var) => self.set_var(var, v, false),
                    None => {
                        if v != Value::Null {
                            self.out.print(&v.to_string());
                        }
                    }
                }
            }
            Cmd::WebKey(e) => {
                let k = self.s(e)?;
                if self.dry(|| format!("WEBKEY {:?}", k)) {
                    return Ok(());
                }
                self.web.key(&k)?;
            }
            Cmd::WebBack => {
                if !self.dry(|| "WEBBACK".into()) {
                    self.web.back()?;
                }
            }
            Cmd::WebReload => {
                if !self.dry(|| "WEBRELOAD".into()) {
                    self.web.reload()?;
                }
            }
            Cmd::WebClose => {
                if !self.dry(|| "WEBCLOSE".into()) {
                    self.web.close();
                }
            }
            Cmd::WebWaitClose(timeout) => {
                let limit = match timeout {
                    Some(x) => Some(Duration::from_millis(self.n(x)?.max(0.0) as u64)),
                    None => None,
                };
                if self.dry(|| "WEBWAITCLOSE".into()) {
                    return Ok(());
                }
                let start = Instant::now();
                while self.web.is_alive() {
                    if limit.is_some_and(|l| start.elapsed() >= l) {
                        return Err(Error::rt("tiempo agotado esperando a que se cierre el navegador".to_string()));
                    }
                    self.sleep_ms(300)?;
                }
            }
        }
        Ok(())
    }
}

// ---------------- Operaciones sobre valores ----------------

/// ¿Es una referencia simple (`x`, `x.y`, `x[i]`)?
fn is_plain_ref(e: &Expr) -> bool {
    match e {
        Expr::Var(_) => true,
        Expr::Member(b, _) | Expr::Index(b, _) => is_plain_ref(b),
        _ => false,
    }
}

impl Interp {
    fn is_defined_ref(&self, e: &Expr) -> bool {
        match e {
            Expr::Var(n) => self.get(n).is_some(),
            Expr::Member(b, _) | Expr::Index(b, _) => self.is_defined_ref(b),
            _ => true,
        }
    }
}

fn length(v: &Value) -> usize {
    match v {
        Value::List(l) => l.len(),
        Value::Null => 0,
        other => other.to_string().chars().count(),
    }
}

fn to_list(v: &Value) -> Vec<Value> {
    match v {
        Value::List(l) => l.clone(),
        Value::Null => vec![],
        other => vec![other.clone()],
    }
}

fn list_index(len: usize, idx: &Value, allow_end: bool) -> R<usize> {
    let n = idx.as_num().ok_or_else(|| Error::rt(format!("índice no numérico: '{}'", idx)))? as i64;
    let i = if n < 0 { len as i64 + n } else { n };
    if i < 0 || (!allow_end && i as usize >= len) {
        return Err(Error::rt(format!("índice {} fuera de rango (tamaño {})", n, len)));
    }
    Ok(i as usize)
}

fn index_value(base: &Value, idx: &Value) -> R<Value> {
    match base {
        Value::List(l) => {
            let i = list_index(l.len(), idx, false)?;
            Ok(l[i].clone())
        }
        Value::Str(s) => {
            let chars: Vec<char> = s.chars().collect();
            let i = list_index(chars.len(), idx, false)?;
            Ok(Value::Str(chars[i].to_string()))
        }
        other => Err(Error::rt(format!("no se puede indexar un valor de tipo {}", other.type_name()))),
    }
}

pub fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::List(x), Value::List(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| values_equal(p, q)),
        _ => match (a.as_num(), b.as_num()) {
            (Some(x), Some(y)) => x == y,
            _ => a.to_string() == b.to_string(),
        },
    }
}

fn binop(op: BinOp, a: &Value, b: &Value) -> R<Value> {
    let nums = || match (a.as_num(), b.as_num()) {
        (Some(x), Some(y)) => Ok((x, y)),
        _ => Err(Error::rt(format!("operación aritmética con valores no numéricos: '{}' y '{}'", a, b))),
    };
    Ok(match op {
        BinOp::Add => match (a, b) {
            (Value::List(x), Value::List(y)) => Value::List(x.iter().chain(y).cloned().collect()),
            (Value::List(x), other) => {
                let mut l = x.clone();
                l.push(other.clone());
                Value::List(l)
            }
            _ => match (a.as_num(), b.as_num()) {
                (Some(x), Some(y)) if !matches!(a, Value::Bool(_)) && !matches!(b, Value::Bool(_)) => Value::Num(x + y),
                _ => Value::Str(format!("{}{}", a, b)),
            },
        },
        BinOp::Sub => {
            let (x, y) = nums()?;
            Value::Num(x - y)
        }
        BinOp::Mul => match (a, b.as_num()) {
            (Value::Str(s), Some(n)) if a.as_num().is_none() => Value::Str(s.repeat(n.max(0.0) as usize)),
            _ => {
                let (x, y) = nums()?;
                Value::Num(x * y)
            }
        },
        BinOp::Div => {
            let (x, y) = nums()?;
            if y == 0.0 {
                return Err(Error::rt("división por cero"));
            }
            Value::Num(x / y)
        }
        BinOp::Mod => {
            let (x, y) = nums()?;
            if y == 0.0 {
                return Err(Error::rt("módulo por cero"));
            }
            Value::Num(x % y)
        }
        BinOp::Eq => Value::Bool(values_equal(a, b)),
        BinOp::Ne => Value::Bool(!values_equal(a, b)),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            let ord = match (a.as_num(), b.as_num()) {
                (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
                _ => a.to_string().cmp(&b.to_string()),
            };
            use std::cmp::Ordering::*;
            Value::Bool(match op {
                BinOp::Lt => ord == Less,
                BinOp::Le => ord != Greater,
                BinOp::Gt => ord == Greater,
                _ => ord != Less,
            })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> Interp {
        let mut it = Interp::new(Options { dry_run: true, ..Default::default() }, Out::default(), Arc::new(AtomicBool::new(false)));
        let prog = parser::parse_program(src).unwrap();
        if let Err(e) = it.exec_block(&prog) {
            panic!("línea {:?}: {}", e.line, e.msg);
        }
        it
    }

    #[test]
    fn arithmetic_and_strings() {
        let it = run(r#"
VAR a = 2 + 3 * 4
VAR b = "192.168.1." + a
VAR c = "${a * 2}-${a - 1}"
VAR d = "=" * 3
VAR start = "5"
VAR e = start + 1
"#);
        assert_eq!(it.get("a"), Some(Value::Num(14.0)));
        assert_eq!(it.get("b"), Some(Value::str("192.168.1.14")));
        assert_eq!(it.get("c"), Some(Value::str("28-13")));
        assert_eq!(it.get("d"), Some(Value::str("===")));
        assert_eq!(it.get("e"), Some(Value::Num(6.0)));
    }

    #[test]
    fn control_flow_and_functions() {
        let it = run(r#"
FUNC fact(n)
  IF n <= 1
    RETURN 1
  ENDIF
  RETURN n * fact(n - 1)
ENDFUNC
VAR total = 0
FOR (VAR i = 10; i > 0; i--)
  total += i
ENDFOR
VAR ips = []
FOR i = 1 TO 3
  ips = push(ips, "10.0.0.${i}")
NEXT
VAR n = 0
WHILE TRUE
  n++
  IF n == 4
    BREAK
  ENDIF
ENDWHILE
VAR f = fact(5)
VAR vacio = ""
VAR ok = EMPTY vacio AND !EMPTY total AND EMPTY nodefinida
"#);
        assert_eq!(it.get("total"), Some(Value::Num(55.0)));
        assert_eq!(it.get("ips").unwrap().to_string(), "10.0.0.1, 10.0.0.2, 10.0.0.3");
        assert_eq!(it.get("f"), Some(Value::Num(120.0)));
        assert_eq!(it.get("ok"), Some(Value::Bool(true)));
    }

    #[test]
    fn conditions_and_interpolation() {
        let it = run(r#"
VAR numero = 20
VAR texto = "Prueba"
VAR IPS = ["a", "b", "c"]
VAR r1 = 0
IF (${numero} < 50)
  r1 = 1
ENDIF
IF (numero == 20 && texto == "Prueba" && IPS.length == 3)
  r1 = r1 + 1
ENDIF
VAR x = IPS[1]
"#);
        assert_eq!(it.get("r1"), Some(Value::Num(2.0)));
        assert_eq!(it.get("x"), Some(Value::str("b")));
    }
}
