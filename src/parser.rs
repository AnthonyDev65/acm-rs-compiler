//! Parser de sentencias y bloques del lenguaje ACM.

use std::rc::Rc;

use crate::expr::{parse_expr_str, parse_template, BinOp, Cursor, Expr};
use crate::lexer::{self, Tok};
use crate::value::Value;

#[derive(Debug)]
pub struct Stmt {
    pub line: usize,
    pub src: String,
    pub repeat: u32,
    pub kind: Kind,
}

#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Target {
    pub name: String,
    pub index: Option<Expr>,
}

#[derive(Debug)]
pub enum Kind {
    Var { name: String, value: Option<Expr> },
    Assign { target: Target, op: Option<BinOp>, value: Expr },
    Expr(Expr),
    If { branches: Vec<(Expr, Vec<Stmt>)>, otherwise: Vec<Stmt> },
    ForIn { var: String, iter: Expr, body: Vec<Stmt> },
    ForC { init: Option<Box<Stmt>>, cond: Option<Expr>, step: Option<Box<Stmt>>, body: Vec<Stmt> },
    While { cond: Expr, body: Vec<Stmt> },
    Loop { count: Option<Expr>, body: Vec<Stmt> },
    Break,
    Continue,
    Return(Option<Expr>),
    Exit(Option<Expr>),
    Func(Rc<FuncDef>),
    Import(Expr),
    Cmd(Cmd),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Btn {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyMode {
    Tap,
    Down,
    Up,
}

#[derive(Debug)]
pub enum TabArg {
    New,
    Close,
    Value(Expr),
}

#[derive(Debug)]
pub enum Cmd {
    Click { button: Btn, count: u8, pos: Option<(Expr, Expr)>, write: Option<Expr> },
    Move(Expr, Expr),
    Drag(Expr, Expr, Expr, Expr),
    Scroll { amount: Expr, horizontal: bool },
    MouseButton { button: Btn, down: bool },
    Write(Expr),
    Paste(Expr),
    Keys { combo: Expr, mode: KeyMode },
    Delay { ms: Expr, mult: f64 },
    TypeDelay(Expr),
    Print(Vec<Expr>),
    Assert(Expr, Option<Expr>),
    Ask { var: String, prompt: Option<Expr> },
    Open(Expr),
    Exec { cmd: Expr, into: Option<String> },
    Alert(Expr),
    Notify(Expr, Option<Expr>),
    WriteFile { path: Expr, content: Expr, append: bool },
    DeleteFile(Expr),
    MkDir(Expr),
    ClipSet(Expr),
    Focus(Expr),
    WaitWindow(Expr, Option<Expr>),
    Screenshot { path: Expr, region: Option<[Expr; 4]> },
    OnError(bool),
    Failsafe(bool),
    WebBrowser(Expr),
    WebProfile(Expr),
    WebTimeout(Expr),
    WebOpen { url: Expr, nowait: bool },
    WebTab { arg: TabArg, nowait: bool },
    WebClick(Expr),
    WebInput(Expr, Expr),
    WebGet(Expr, String),
    WebWait { sel: Expr, cond: String, timeout: Option<Expr> },
    WebSelect(Expr, Expr),
    WebCheck(Expr, Expr),
    WebScroll(Expr),
    WebScreenshot(Expr),
    WebJs { code: Expr, into: Option<String> },
    WebKey(Expr),
    WebBack,
    WebReload,
    WebClose,
}

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub msg: String,
    /// El código termina dentro de un bloque abierto (útil para el REPL).
    pub incomplete: bool,
}

#[derive(Clone)]
struct Raw {
    line: usize,
    text: String,
}

const TERMINATORS: &[&str] = &[
    "ENDIF", "ELSE", "ELSEIF", "ENDFOR", "NEXT", "ENDWHILE", "ENDLOOP", "ENDFUNC", "END",
];

/// Comandos a los que se puede aplicar el sufijo de repetición `(N)`.
const REPEATABLE: &[&str] = &[
    "PRESS", "CLICK", "RIGHTCLICK", "DOUBLECLICK", "MIDDLECLICK", "DRAG", "MOVE", "WRITE", "TYPE",
    "TEXT", "PASTE", "KEY", "KEYPRESS", "PRESSKEY", "HOTKEY", "SCROLL", "DELAY", "WAIT", "SLEEP",
    "NOTIFY", "ALERT", "OPEN", "EXEC", "RUN", "SHELL", "WEBCLICK", "WEBKEY", "WEBTAB", "WEBSCROLL",
];

pub fn parse_program(src: &str) -> Result<Vec<Stmt>, ParseError> {
    let raws = split_source(src);
    let mut p = Parser { raws, i: 0 };
    let (stmts, term) = p.block(&[])?;
    if let Some((kw, raw)) = term {
        return Err(ParseError { line: raw.line, msg: format!("{} inesperado", kw), incomplete: false });
    }
    Ok(stmts)
}

fn split_source(src: &str) -> Vec<Raw> {
    let mut out = Vec::new();
    for (n, line) in src.lines().enumerate() {
        let t = line.trim().trim_start_matches('\u{feff}');
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') || t.starts_with('@') {
            continue;
        }
        for s in lexer::split_statements(t) {
            out.push(Raw { line: n + 1, text: s });
        }
    }
    out
}

/// Separa la primera palabra (en mayúsculas) del resto.
fn head(text: &str) -> (String, &str) {
    let end = text
        .char_indices()
        .find(|(_, c)| c.is_whitespace() || *c == '(' || *c == '"')
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    (text[..end].to_ascii_uppercase(), text[end..].trim())
}

/// Normaliza una sentencia: llaves `{ }`, etiquetas `A:`.
fn normalize(text: &str) -> String {
    let mut t = text.trim().to_string();
    if t.starts_with('}') && t.len() > 1 {
        t = t[1..].trim().to_string();
    }
    if t.ends_with('{') && t.len() > 1 {
        t = t[..t.len() - 1].trim().to_string();
    }
    // Etiqueta estilo "A:PRESS 100 200"
    if let Some(p) = t.find(':') {
        let sp = t.find(char::is_whitespace).unwrap_or(t.len());
        let prefix = &t[..p];
        let rest = &t[p + 1..];
        if p > 0
            && p < sp
            && prefix.chars().all(lexer::is_ident_char)
            && !rest.is_empty()
            && !rest.starts_with(['/', '\\', ':'])
        {
            t = rest.trim().to_string();
        }
    }
    t
}

fn terminator_of(text: &str) -> Option<String> {
    if text == "}" {
        return Some("END".into());
    }
    let (kw, rest) = head(text);
    if kw == "ELSE" && head(rest).0 == "IF" {
        return Some("ELSEIF".into());
    }
    if TERMINATORS.contains(&kw.as_str()) { Some(kw) } else { None }
}

struct Parser {
    raws: Vec<Raw>,
    i: usize,
}

impl Parser {
    fn block(&mut self, terms: &[&str]) -> Result<(Vec<Stmt>, Option<(String, Raw)>), ParseError> {
        let mut stmts = Vec::new();
        while self.i < self.raws.len() {
            let raw = self.raws[self.i].clone();
            let text = normalize(&raw.text);
            let original_brace = raw.text.trim_start().starts_with('}');
            if text == "}" || (!text.is_empty() && terminator_of(&text).is_some()) {
                let kw = terminator_of(&text).unwrap();
                self.i += 1;
                let accepted = kw == "END" || terms.contains(&kw.as_str());
                if !accepted {
                    let expected = if terms.is_empty() { String::new() } else { format!(" (se esperaba {})", terms.join(" o ")) };
                    return Err(ParseError { line: raw.line, msg: format!("{} inesperado{}", kw, expected), incomplete: false });
                }
                return Ok((stmts, Some((kw, Raw { line: raw.line, text }))));
            }
            if original_brace && text.is_empty() {
                self.i += 1;
                continue;
            }
            self.i += 1;
            let stmt = self.statement(&raw, &text)?;
            stmts.push(stmt);
        }
        Ok((stmts, None))
    }

    fn err(line: usize, msg: impl Into<String>) -> ParseError {
        ParseError { line, msg: msg.into(), incomplete: false }
    }

    fn unclosed(line: usize, what: &str, end: &str) -> ParseError {
        ParseError { line, msg: format!("{} sin {}", what, end), incomplete: true }
    }

    fn statement(&mut self, raw: &Raw, text: &str) -> Result<Stmt, ParseError> {
        let line = raw.line;
        let (kw, rest) = head(text);
        let mk = |kind: Kind| Stmt { line, src: text.to_string(), repeat: 1, kind };
        match kw.as_str() {
            "IF" => {
                let mut branches = Vec::new();
                let mut cond = cond_expr(rest).map_err(|e| Self::err(line, e))?;
                let otherwise;
                loop {
                    let (body, term) = self.block(&["ELSE", "ELSEIF", "ENDIF"])?;
                    branches.push((cond, body));
                    match term {
                        Some((k, r)) if k == "ELSEIF" => {
                            let (_, after) = head(&r.text);
                            let after = if head(after).0 == "IF" { head(after).1 } else { after };
                            cond = cond_expr(after).map_err(|e| Self::err(r.line, e))?;
                        }
                        Some((k, _)) if k == "ELSE" => {
                            let (body, term) = self.block(&["ENDIF"])?;
                            if term.is_none() {
                                return Err(Self::unclosed(line, "IF", "ENDIF"));
                            }
                            otherwise = body;
                            break;
                        }
                        Some(_) => {
                            otherwise = Vec::new();
                            break;
                        }
                        None => return Err(Self::unclosed(line, "IF", "ENDIF")),
                    }
                }
                Ok(mk(Kind::If { branches, otherwise }))
            }
            "WHILE" | "MIENTRAS" => {
                let cond = cond_expr(rest).map_err(|e| Self::err(line, e))?;
                let (body, term) = self.block(&["ENDWHILE"])?;
                if term.is_none() {
                    return Err(Self::unclosed(line, "WHILE", "ENDWHILE"));
                }
                Ok(mk(Kind::While { cond, body }))
            }
            "FOR" | "FOREACH" => self.for_stmt(line, text, rest),
            "LOOP" | "REPEAT" => {
                let up = rest.trim().to_ascii_uppercase();
                let count = if up.is_empty() || up == "INFINITE" || up == "INFINITO" || up == "FOREVER" {
                    None
                } else {
                    Some(parse_expr_str(rest).map_err(|e| Self::err(line, e))?)
                };
                // Sin ENDLOOP, el bucle abarca el resto del script.
                let (body, _) = self.block(&["ENDLOOP"])?;
                Ok(mk(Kind::Loop { count, body }))
            }
            "FUNC" | "FUNCTION" | "DEF" => {
                let mut c = Cursor::new(rest).map_err(|e| Self::err(line, e))?;
                let name = c.ident().map_err(|e| Self::err(line, e))?;
                let mut params = Vec::new();
                if c.eat_op("(") && !c.eat_op(")") {
                    loop {
                        params.push(c.ident().map_err(|e| Self::err(line, e))?);
                        if c.eat_op(")") {
                            break;
                        }
                        c.expect_op(",").map_err(|e| Self::err(line, e))?;
                    }
                }
                c.expect_end().map_err(|e| Self::err(line, e))?;
                let (body, term) = self.block(&["ENDFUNC"])?;
                if term.is_none() {
                    return Err(Self::unclosed(line, "FUNC", "ENDFUNC"));
                }
                Ok(mk(Kind::Func(Rc::new(FuncDef { name, params, body }))))
            }
            _ => simple(line, text),
        }
    }

    fn for_stmt(&mut self, line: usize, text: &str, rest: &str) -> Result<Stmt, ParseError> {
        let e = |m: String| Self::err(line, m);
        let inner = rest.trim();
        let inner = if inner.starts_with('(') && inner.ends_with(')') { inner[1..inner.len() - 1].trim() } else { inner };
        let parts = split_semicolons(inner);
        let kind_partial: Box<dyn FnOnce(Vec<Stmt>) -> Kind>;
        if parts.len() == 3 {
            let init = if parts[0].is_empty() { None } else { Some(Box::new(simple(line, &parts[0])?)) };
            let cond = if parts[1].is_empty() { None } else { Some(parse_expr_str(&parts[1]).map_err(e)?) };
            let step = if parts[2].is_empty() { None } else { Some(Box::new(simple(line, &parts[2])?)) };
            kind_partial = Box::new(move |body| Kind::ForC { init, cond, step, body });
        } else {
            let mut c = Cursor::new(inner).map_err(e)?;
            c.eat_kw("VAR");
            let var = c.ident().map_err(|m| Self::err(line, m))?;
            if c.eat_kw("IN") || c.eat_kw("EN") {
                let iter = c.expr().map_err(|m| Self::err(line, m))?;
                c.expect_end().map_err(|m| Self::err(line, m))?;
                kind_partial = Box::new(move |body| Kind::ForIn { var, iter, body });
            } else if c.eat_op("=") {
                // FOR i = 1 TO 10 [STEP 2]
                let from = c.expr().map_err(|m| Self::err(line, m))?;
                if !c.eat_kw("TO") && !c.eat_kw("HASTA") {
                    return Err(Self::err(line, "se esperaba TO en FOR i = a TO b"));
                }
                let to = c.expr().map_err(|m| Self::err(line, m))?;
                let step = if c.eat_kw("STEP") || c.eat_kw("PASO") { Some(c.expr().map_err(|m| Self::err(line, m))?) } else { None };
                c.expect_end().map_err(|m| Self::err(line, m))?;
                let iter = Expr::Call("__range_incl".into(), vec![from, to, step.unwrap_or(Expr::Lit(Value::Num(1.0)))]);
                kind_partial = Box::new(move |body| Kind::ForIn { var, iter, body });
            } else {
                return Err(Self::err(line, "FOR inválido: usa FOR x IN lista, FOR (VAR i = 0; i < n; i++) o FOR i = 1 TO n"));
            }
        }
        let (body, term) = self.block(&["ENDFOR", "NEXT"])?;
        if term.is_none() {
            return Err(Self::unclosed(line, "FOR", "ENDFOR"));
        }
        Ok(Stmt { line, src: text.to_string(), repeat: 1, kind: kind_partial(body) })
    }
}

fn split_semicolons(s: &str) -> Vec<String> {
    let c: Vec<char> = s.chars().collect();
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut i = 0;
    let mut depth = 0;
    while i < c.len() {
        let ch = c[i];
        if ch == '"' {
            if let Some(e) = lexer::skip_dq(&c, i + 1) {
                cur.extend(&c[i..=e]);
                i = e + 1;
                continue;
            }
        }
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ';' if depth == 0 => {
                parts.push(cur.trim().to_string());
                cur.clear();
                i += 1;
                continue;
            }
            _ => {}
        }
        cur.push(ch);
        i += 1;
    }
    parts.push(cur.trim().to_string());
    parts
}

fn cond_expr(rest: &str) -> Result<Expr, String> {
    let t = rest.trim();
    let t = t.strip_suffix("THEN").or_else(|| t.strip_suffix("then")).unwrap_or(t).trim();
    if t.is_empty() {
        return Err("falta la condición".into());
    }
    parse_expr_str(t)
}

/// Extrae el sufijo de repetición ` (N)` al final del texto.
fn extract_repeat(text: &str) -> (String, u32) {
    let t = text.trim_end();
    if t.ends_with(')') {
        if let Some(open) = t.rfind('(') {
            let inside = &t[open + 1..t.len() - 1];
            let before = &t[..open];
            if !inside.is_empty()
                && inside.bytes().all(|b| b.is_ascii_digit())
                && before.ends_with(char::is_whitespace)
            {
                if let Ok(n) = inside.parse::<u32>() {
                    return (before.trim_end().to_string(), n.max(1));
                }
            }
        }
    }
    (t.to_string(), 1)
}

/// Extrae `(IGNORE)` / `(NOWAIT)` del principio o del final de los argumentos.
fn extract_nowait(rest: &str) -> (String, bool) {
    let t = rest.trim();
    for tag in ["(IGNORE)", "(NOWAIT)"] {
        if t.len() >= tag.len() && t[..tag.len()].eq_ignore_ascii_case(tag) {
            return (t[tag.len()..].trim().to_string(), true);
        }
        if t.len() >= tag.len() && t[t.len() - tag.len()..].eq_ignore_ascii_case(tag) {
            return (t[..t.len() - tag.len()].trim().to_string(), true);
        }
    }
    (t.to_string(), false)
}

fn into_target(c: &mut Cursor) -> Result<Option<String>, String> {
    if c.eat_op("->") || c.eat_kw("INTO") || c.eat_kw("EN") {
        Ok(Some(c.ident()?))
    } else {
        Ok(None)
    }
}

fn button_kw(c: &mut Cursor) -> Btn {
    if c.eat_kw("RIGHT") || c.eat_kw("DERECHO") {
        Btn::Right
    } else if c.eat_kw("MIDDLE") || c.eat_kw("CENTRO") {
        Btn::Middle
    } else {
        c.eat_kw("LEFT");
        c.eat_kw("IZQUIERDO");
        Btn::Left
    }
}

fn on_off(rest: &str) -> Result<bool, String> {
    match rest.trim().to_ascii_uppercase().as_str() {
        "ON" | "TRUE" | "1" | "SI" | "CONTINUE" => Ok(true),
        "OFF" | "FALSE" | "0" | "NO" | "STOP" => Ok(false),
        other => Err(format!("valor no válido '{}' (usa ON/OFF)", other)),
    }
}

fn delay_expr(rest: &str) -> Result<(Expr, f64), String> {
    let t = rest.trim();
    let lower = t.to_ascii_lowercase();
    for (suffix, mult) in [("ms", 1.0), ("min", 60000.0), ("s", 1000.0)] {
        if lower.ends_with(suffix) {
            let body = t[..t.len() - suffix.len()].trim_end();
            let prev = body.chars().last();
            let unit_ok = match prev {
                Some(ch) => ch.is_ascii_digit() || ch == '}' || ch == ')' || t[body.len()..].starts_with(char::is_whitespace),
                None => false,
            };
            if unit_ok {
                return Ok((parse_expr_str(body)?, mult));
            }
        }
    }
    Ok((parse_expr_str(t)?, 1.0))
}

/// Parsea una sentencia simple (sin bloque).
pub fn simple(line: usize, text: &str) -> Result<Stmt, ParseError> {
    let (kw, _) = head(text);
    let (text_norep, repeat) = if REPEATABLE.contains(&kw.as_str()) { extract_repeat(text) } else { (text.to_string(), 1) };
    let (_, rest) = head(&text_norep);
    let kind = simple_kind(&kw, rest, &text_norep).map_err(|msg| ParseError { line, msg, incomplete: false })?;
    Ok(Stmt { line, src: text.to_string(), repeat, kind })
}

fn simple_kind(kw: &str, rest: &str, full: &str) -> Result<Kind, String> {
    let cur = || Cursor::new(rest);
    let one = || -> Result<Expr, String> {
        if rest.trim().is_empty() {
            return Err(format!("{} necesita un argumento", kw));
        }
        parse_expr_str(rest)
    };
    let cmd = |c: Cmd| Ok(Kind::Cmd(c));
    match kw {
        "VAR" | "LET" | "SET" | "CONST" => {
            let t = rest.trim();
            let (name, value) = match t.find('=') {
                Some(eq) if !t[eq..].starts_with("==") => (t[..eq].trim(), Some(t[eq + 1..].trim())),
                _ => (t, None),
            };
            if name.is_empty() || !name.chars().all(lexer::is_ident_char) {
                return Err(format!("nombre de variable no válido '{}'", name));
            }
            let value = match value {
                None => None,
                Some(v) if v.is_empty() => Some(Expr::Lit(Value::str(""))),
                Some(v) => Some(match parse_expr_str(v) {
                    Ok(e) => e,
                    Err(err) => {
                        // Texto sin comillas (rutas, palabras sueltas) se toma literal.
                        let bare = lexer::lex(v)
                            .map(|t| t.iter().all(|t| matches!(t.tok, Tok::Ident(_) | Tok::Num(_))))
                            .unwrap_or(true);
                        if bare {
                            Expr::Lit(Value::str(v))
                        } else {
                            return Err(err);
                        }
                    }
                }),
            };
            Ok(Kind::Var { name: name.to_string(), value })
        }
        "PRESS" | "CLICK" | "DOUBLECLICK" | "RIGHTCLICK" | "MIDDLECLICK" => {
            let mut c = cur()?;
            let (button, count) = match kw {
                "DOUBLECLICK" => (Btn::Left, 2),
                "RIGHTCLICK" => (Btn::Right, 1),
                "MIDDLECLICK" => (Btn::Middle, 1),
                _ => (Btn::Left, 1),
            };
            let pos = if c.at_end() || c.is_kw("WRITE") || c.is_kw("TYPE") {
                None
            } else {
                let x = c.expr()?;
                c.sep();
                let y = c.expr()?;
                Some((x, y))
            };
            let write = if c.eat_kw("WRITE") || c.eat_kw("TYPE") { Some(c.expr()?) } else { None };
            c.expect_end()?;
            cmd(Cmd::Click { button, count, pos, write })
        }
        "MOVE" | "MOUSEMOVE" => {
            let mut c = cur()?;
            let x = c.expr()?;
            c.sep();
            let y = c.expr()?;
            c.expect_end()?;
            cmd(Cmd::Move(x, y))
        }
        "DRAG" => {
            let mut c = cur()?;
            let x1 = c.expr()?;
            c.sep();
            let y1 = c.expr()?;
            if !c.eat_kw("TO") && !c.eat_op("->") {
                c.sep();
            }
            let x2 = c.expr()?;
            c.sep();
            let y2 = c.expr()?;
            c.expect_end()?;
            cmd(Cmd::Drag(x1, y1, x2, y2))
        }
        "SCROLL" => {
            let mut c = cur()?;
            let mut sign = 1.0;
            let mut horizontal = false;
            if c.eat_kw("UP") || c.eat_kw("ARRIBA") {
                sign = -1.0;
            } else if c.eat_kw("DOWN") || c.eat_kw("ABAJO") {
            } else if c.eat_kw("LEFT") || c.eat_kw("IZQUIERDA") {
                sign = -1.0;
                horizontal = true;
            } else if c.eat_kw("RIGHT") || c.eat_kw("DERECHA") {
                horizontal = true;
            }
            let amount = if c.at_end() { Expr::Lit(Value::Num(3.0)) } else { c.expr()? };
            if c.eat_kw("HORIZONTAL") || c.eat_kw("H") {
                horizontal = true;
            }
            c.expect_end()?;
            let amount = if sign < 0.0 { Expr::Unary(crate::expr::UnOp::Neg, Box::new(amount)) } else { amount };
            cmd(Cmd::Scroll { amount, horizontal })
        }
        "MOUSEDOWN" | "MOUSEUP" => {
            let mut c = cur()?;
            let button = button_kw(&mut c);
            c.expect_end()?;
            cmd(Cmd::MouseButton { button, down: kw == "MOUSEDOWN" })
        }
        "WRITE" | "TYPE" | "TEXT" | "ESCRIBIR" => cmd(Cmd::Write(one()?)),
        "PASTE" | "PEGAR" => cmd(Cmd::Paste(one()?)),
        "KEY" | "KEYPRESS" | "PRESSKEY" | "HOTKEY" | "KEYDOWN" | "KEYUP" | "TECLA" => {
            if rest.trim().is_empty() {
                return Err(format!("{} necesita una tecla (ej: {} CTRL + C)", kw, kw));
            }
            let mode = match kw {
                "KEYDOWN" => KeyMode::Down,
                "KEYUP" => KeyMode::Up,
                _ => KeyMode::Tap,
            };
            let raw = rest.trim();
            let raw = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')).unwrap_or(raw);
            cmd(Cmd::Keys { combo: parse_template(raw)?, mode })
        }
        "DELAY" | "WAIT" | "SLEEP" | "ESPERAR" => {
            if rest.trim().is_empty() {
                return Err("DELAY necesita una duración (ej: DELAY 500ms)".into());
            }
            let (ms, mult) = delay_expr(rest)?;
            cmd(Cmd::Delay { ms, mult })
        }
        "TYPEDELAY" => cmd(Cmd::TypeDelay(one()?)),
        "PRINT" | "ECHO" | "LOG" | "IMPRIMIR" => {
            let mut c = cur()?;
            let mut items = Vec::new();
            while !c.at_end() {
                items.push(c.expr()?);
                c.sep();
            }
            cmd(Cmd::Print(items))
        }
        "ASSERT" => {
            let mut c = cur()?;
            let cond = c.expr()?;
            c.sep();
            let msg = if c.at_end() { None } else { Some(c.expr()?) };
            c.expect_end()?;
            cmd(Cmd::Assert(cond, msg))
        }
        "ASK" | "INPUT" | "PREGUNTAR" => {
            let mut c = cur()?;
            let var = c.ident()?;
            c.sep();
            let prompt = if c.at_end() { None } else { Some(c.expr()?) };
            c.expect_end()?;
            cmd(Cmd::Ask { var, prompt })
        }
        "OPEN" | "ABRIR" | "START" => cmd(Cmd::Open(one()?)),
        "EXEC" | "RUN" | "SHELL" | "EJECUTAR" => {
            let mut c = cur()?;
            let command = c.expr()?;
            let into = into_target(&mut c)?;
            c.expect_end()?;
            cmd(Cmd::Exec { cmd: command, into })
        }
        "ALERT" | "ALERTA" => cmd(Cmd::Alert(one()?)),
        "NOTIFY" | "NOTIFICAR" => {
            let mut c = cur()?;
            let msg = c.expr()?;
            c.sep();
            let title = if c.at_end() { None } else { Some(c.expr()?) };
            c.expect_end()?;
            cmd(Cmd::Notify(msg, title))
        }
        "WRITEFILE" | "APPENDFILE" => {
            let mut c = cur()?;
            let path = c.expr()?;
            c.sep();
            let content = c.expr()?;
            c.expect_end()?;
            cmd(Cmd::WriteFile { path, content, append: kw == "APPENDFILE" })
        }
        "DELETEFILE" => cmd(Cmd::DeleteFile(one()?)),
        "MKDIR" => cmd(Cmd::MkDir(one()?)),
        "CLIPSET" | "SETCLIPBOARD" | "CLIPBOARD" => cmd(Cmd::ClipSet(one()?)),
        "FOCUS" | "ACTIVATE" => cmd(Cmd::Focus(one()?)),
        "WAITWINDOW" => {
            let mut c = cur()?;
            let title = c.expr()?;
            c.sep();
            let timeout = if c.at_end() { None } else { Some(c.expr()?) };
            c.expect_end()?;
            cmd(Cmd::WaitWindow(title, timeout))
        }
        "SCREENSHOT" | "CAPTURE" | "CAPTURA" => {
            let mut c = cur()?;
            let path = c.expr()?;
            c.sep();
            let region = if c.at_end() {
                None
            } else {
                let x = c.expr()?;
                c.sep();
                let y = c.expr()?;
                c.sep();
                let w = c.expr()?;
                c.sep();
                let h = c.expr()?;
                Some([x, y, w, h])
            };
            c.expect_end()?;
            cmd(Cmd::Screenshot { path, region })
        }
        "ONERROR" => cmd(Cmd::OnError(on_off(rest)?)),
        "FAILSAFE" => cmd(Cmd::Failsafe(on_off(rest)?)),
        "BREAK" => Ok(Kind::Break),
        "CONTINUE" => Ok(Kind::Continue),
        "RETURN" => Ok(Kind::Return(if rest.trim().is_empty() { None } else { Some(parse_expr_str(rest)?) })),
        "EXIT" | "QUIT" | "SALIR" => Ok(Kind::Exit(if rest.trim().is_empty() { None } else { Some(parse_expr_str(rest)?) })),
        "IMPORT" | "INCLUDE" => Ok(Kind::Import(one()?)),
        "WEBBROWSER" | "BROWSER" => {
            cmd(Cmd::WebBrowser(parse_template(rest.trim())?))
        }
        "WEBPROFILE" => cmd(Cmd::WebProfile(one()?)),
        "WEBTIMEOUT" => cmd(Cmd::WebTimeout(one()?)),
        "WEBOPEN" | "WEBGOTO" | "WEBNAV" => {
            let (r, nowait) = extract_nowait(rest);
            if r.is_empty() {
                return Err("WEBOPEN necesita una URL".into());
            }
            cmd(Cmd::WebOpen { url: parse_expr_str(&r)?, nowait })
        }
        "WEBTAB" => {
            let (r, nowait) = extract_nowait(rest);
            let up = r.to_ascii_uppercase();
            let arg = match up.as_str() {
                "" | "NEW" | "NUEVA" => TabArg::New,
                "CLOSE" | "CERRAR" => TabArg::Close,
                _ => TabArg::Value(parse_expr_str(&r)?),
            };
            cmd(Cmd::WebTab { arg, nowait })
        }
        "WEBCLICK" => cmd(Cmd::WebClick(one()?)),
        "WEBSCROLL" => cmd(Cmd::WebScroll(one()?)),
        "WEBSCREENSHOT" => cmd(Cmd::WebScreenshot(one()?)),
        "WEBKEY" => cmd(Cmd::WebKey(one()?)),
        "WEBBACK" => cmd(Cmd::WebBack),
        "WEBRELOAD" => cmd(Cmd::WebReload),
        "WEBCLOSE" => cmd(Cmd::WebClose),
        "WEBINPUT" | "WEBSELECT" | "WEBCHECK" => {
            let mut c = cur()?;
            let sel = c.expr()?;
            c.sep();
            let val = c.expr()?;
            c.expect_end()?;
            cmd(match kw {
                "WEBINPUT" => Cmd::WebInput(sel, val),
                "WEBSELECT" => Cmd::WebSelect(sel, val),
                _ => Cmd::WebCheck(sel, val),
            })
        }
        "WEBGET" => {
            let mut c = cur()?;
            let sel = c.expr()?;
            c.sep();
            c.eat_op("->");
            c.eat_kw("INTO");
            let var = c.ident()?;
            c.expect_end()?;
            cmd(Cmd::WebGet(sel, var))
        }
        "WEBWAIT" => {
            let mut c = cur()?;
            let sel = c.expr()?;
            let mut cond = "EXISTS".to_string();
            if let Some(Tok::Ident(s)) = c.peek() {
                let up = s.to_ascii_uppercase();
                if ["EXISTS", "VISIBLE", "HIDDEN", "CLICKABLE", "GONE"].contains(&up.as_str()) {
                    cond = up;
                    c.pos += 1;
                }
            }
            c.sep();
            let timeout = if c.at_end() { None } else { Some(c.expr()?) };
            c.expect_end()?;
            cmd(Cmd::WebWait { sel, cond, timeout })
        }
        "WEBJS" | "WEBEVAL" => {
            let mut c = cur()?;
            let code = c.expr()?;
            let into = into_target(&mut c)?;
            c.expect_end()?;
            cmd(Cmd::WebJs { code, into })
        }
        _ => assignment_or_expr(full),
    }
}

fn assignment_or_expr(full: &str) -> Result<Kind, String> {
    let mut c = Cursor::new(full)?;
    if let Some(Tok::Ident(name)) = c.peek().cloned() {
        let save = c.pos;
        c.pos += 1;
        let index = if c.is_op("[") {
            c.pos += 1;
            let e = c.expr()?;
            c.expect_op("]")?;
            Some(e)
        } else {
            None
        };
        let target = Target { name: name.clone(), index };
        let op = match c.peek() {
            Some(Tok::Op("=")) => Some(None),
            Some(Tok::Op("+=")) => Some(Some(BinOp::Add)),
            Some(Tok::Op("-=")) => Some(Some(BinOp::Sub)),
            Some(Tok::Op("*=")) => Some(Some(BinOp::Mul)),
            Some(Tok::Op("/=")) => Some(Some(BinOp::Div)),
            Some(Tok::Op("++")) | Some(Tok::Op("--")) => {
                let add = c.is_op("++");
                c.pos += 1;
                c.expect_end()?;
                return Ok(Kind::Assign {
                    target,
                    op: Some(if add { BinOp::Add } else { BinOp::Sub }),
                    value: Expr::Lit(Value::Num(1.0)),
                });
            }
            _ => None,
        };
        if let Some(op) = op {
            c.pos += 1;
            let value = c.expr()?;
            c.expect_end()?;
            return Ok(Kind::Assign { target, op, value });
        }
        c.pos = save;
    }
    let e = c.expr().map_err(|e| {
        let (kw, _) = head(full);
        if kw.chars().all(|ch| ch.is_ascii_uppercase()) && kw.len() > 1 {
            format!("comando desconocido '{}' o expresión inválida: {}", kw, e)
        } else {
            e
        }
    })?;
    c.expect_end().map_err(|e| {
        let (kw, _) = head(full);
        if kw.chars().all(|ch| ch.is_ascii_uppercase()) && kw.len() > 1 {
            format!("comando desconocido '{}'", kw)
        } else {
            e
        }
    })?;
    Ok(Kind::Expr(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_blocks() {
        let src = r#"
VAR n = 20
FOR (VAR i = 0; i < 3; i++)
    IF (${n} < 50 AND i == 1)
        WRITE "x ${i}"
    ELSE IF i == 2
        KEYPRESS CTRL + C
    ELSE
        DELAY 100ms
    ENDIF
ENDFOR
WRITE "=" (5)
A:PRESS 300 200
WEBOPEN(IGNORE) "http://x"
WEBTAB 0  // comentario
"#;
        let p = parse_program(src).unwrap();
        assert_eq!(p.len(), 6);
        assert_eq!(p[2].repeat, 5);
    }

    #[test]
    fn unclosed_is_incomplete() {
        let e = parse_program("IF x == 1\nWRITE \"a\"").unwrap_err();
        assert!(e.incomplete);
    }

    #[test]
    fn loop_compat() {
        let p = parse_program("LOOP 3\nCLICK 1 2\nDELAY 10").unwrap();
        assert_eq!(p.len(), 1);
    }

    #[test]
    fn unknown_command() {
        assert!(parse_program("FOOBAR 1 2").is_err());
    }
}
