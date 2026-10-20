//! AST y parser de expresiones (con precedencia).

use crate::lexer::{self, Tok, Token};
use crate::value::Value;

#[derive(Clone, Debug)]
pub enum Expr {
    Lit(Value),
    Tpl(Vec<Part>),
    Var(String),
    List(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Member(Box<Expr>, String),
    Call(String, Vec<Expr>),
    Unary(UnOp, Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Empty(Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum Part {
    Text(String),
    /// Expresión interpolada y su texto original (para dejarla tal cual si no se puede resolver).
    Expr(Expr, String),
}

#[derive(Clone, Copy, Debug)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Cursor sobre una lista de tokens; los comandos lo usan para leer varios argumentos seguidos.
pub struct Cursor {
    pub toks: Vec<Token>,
    pub pos: usize,
}

impl Cursor {
    pub fn new(src: &str) -> Result<Self, String> {
        Ok(Cursor { toks: lexer::lex(src)?, pos: 0 })
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.toks.len()
    }

    pub fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.tok)
    }

    pub fn peek_at(&self, n: usize) -> Option<&Tok> {
        self.toks.get(self.pos + n).map(|t| &t.tok)
    }

    pub fn is_op(&self, op: &str) -> bool {
        matches!(self.peek(), Some(Tok::Op(o)) if *o == op)
    }

    pub fn eat_op(&mut self, op: &str) -> bool {
        if self.is_op(op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    pub fn expect_op(&mut self, op: &str) -> Result<(), String> {
        if self.eat_op(op) { Ok(()) } else { Err(format!("se esperaba '{}'", op)) }
    }

    pub fn is_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(s)) if s.eq_ignore_ascii_case(kw))
    }

    pub fn eat_kw(&mut self, kw: &str) -> bool {
        if self.is_kw(kw) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    pub fn ident(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(Tok::Ident(s)) => {
                let s = s.clone();
                self.pos += 1;
                Ok(s)
            }
            _ => Err("se esperaba un nombre de variable".into()),
        }
    }

    pub fn expect_end(&self) -> Result<(), String> {
        if self.at_end() {
            Ok(())
        } else {
            Err(format!("texto inesperado: {}", self.describe_rest()))
        }
    }

    fn describe_rest(&self) -> String {
        match self.peek() {
            Some(Tok::Ident(s)) => s.clone(),
            Some(Tok::Op(o)) => o.to_string(),
            Some(Tok::Num(n)) => n.to_string(),
            Some(Tok::Str(s)) => format!("\"{}\"", s),
            Some(Tok::RawStr(s)) => format!("'{}'", s),
            Some(Tok::Interp(s)) => format!("${{{}}}", s),
            None => String::new(),
        }
    }

    /// Consume una coma opcional entre argumentos.
    pub fn sep(&mut self) {
        self.eat_op(",");
    }

    pub fn expr(&mut self) -> Result<Expr, String> {
        if self.at_end() {
            return Err("se esperaba una expresión".into());
        }
        self.or()
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut l = self.and()?;
        while self.eat_op("||") || self.eat_kw("OR") {
            let r = self.and()?;
            l = Expr::Or(Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut l = self.not()?;
        while self.eat_op("&&") || self.eat_kw("AND") {
            let r = self.not()?;
            l = Expr::And(Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn not(&mut self) -> Result<Expr, String> {
        if self.eat_kw("NOT") {
            let e = self.not()?;
            return Ok(Expr::Unary(UnOp::Not, Box::new(e)));
        }
        self.cmp()
    }

    fn cmp(&mut self) -> Result<Expr, String> {
        let mut l = self.add()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op("==")) => BinOp::Eq,
                Some(Tok::Op("!=")) => BinOp::Ne,
                Some(Tok::Op("<")) => BinOp::Lt,
                Some(Tok::Op("<=")) => BinOp::Le,
                Some(Tok::Op(">")) => BinOp::Gt,
                Some(Tok::Op(">=")) => BinOp::Ge,
                // `=` dentro de una condición equivale a `==`
                Some(Tok::Op("=")) => BinOp::Eq,
                _ => break,
            };
            self.pos += 1;
            let r = self.add()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn add(&mut self) -> Result<Expr, String> {
        let mut l = self.mul()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op("+")) => BinOp::Add,
                Some(Tok::Op("-")) => BinOp::Sub,
                _ => break,
            };
            self.pos += 1;
            let r = self.mul()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn mul(&mut self) -> Result<Expr, String> {
        let mut l = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op("*")) => BinOp::Mul,
                Some(Tok::Op("/")) => BinOp::Div,
                Some(Tok::Op("%")) => BinOp::Mod,
                _ => break,
            };
            self.pos += 1;
            let r = self.unary()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat_op("-") {
            return Ok(Expr::Unary(UnOp::Neg, Box::new(self.unary()?)));
        }
        if self.eat_op("!") {
            return Ok(Expr::Unary(UnOp::Not, Box::new(self.unary()?)));
        }
        if self.eat_kw("EMPTY") {
            return Ok(Expr::Empty(Box::new(self.unary()?)));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.primary()?;
        loop {
            if self.is_op("[") && self.adjacent() {
                self.pos += 1;
                let idx = self.expr()?;
                self.expect_op("]")?;
                e = Expr::Index(Box::new(e), Box::new(idx));
            } else if self.is_op(".") && matches!(self.peek_at(1), Some(Tok::Ident(_))) {
                self.pos += 1;
                let name = self.ident()?;
                if self.is_op("(") && self.adjacent() {
                    self.pos += 1;
                    let mut args = vec![e];
                    args.extend(self.args()?);
                    e = Expr::Call(name, args);
                } else {
                    e = Expr::Member(Box::new(e), name);
                }
            } else {
                break;
            }
        }
        Ok(e)
    }

    /// ¿El token actual está pegado al anterior (sin espacios)?
    fn adjacent(&self) -> bool {
        self.pos > 0 && self.toks[self.pos].start == self.toks[self.pos - 1].end
    }

    fn args(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        if self.eat_op(")") {
            return Ok(args);
        }
        loop {
            args.push(self.expr()?);
            if self.eat_op(")") {
                return Ok(args);
            }
            self.expect_op(",")?;
        }
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let t = self.toks.get(self.pos).cloned().ok_or("expresión incompleta")?;
        self.pos += 1;
        match t.tok {
            Tok::Num(n) => Ok(Expr::Lit(Value::Num(n))),
            Tok::Str(s) => parse_template(&s),
            Tok::RawStr(s) => Ok(Expr::Lit(Value::Str(s))),
            Tok::Interp(src) => parse_expr_str(&src),
            Tok::Ident(name) => {
                let up = name.to_ascii_uppercase();
                match up.as_str() {
                    "TRUE" | "VERDADERO" => return Ok(Expr::Lit(Value::Bool(true))),
                    "FALSE" | "FALSO" => return Ok(Expr::Lit(Value::Bool(false))),
                    "NULL" | "NIL" => return Ok(Expr::Lit(Value::Null)),
                    _ => {}
                }
                if self.is_op("(") && self.adjacent() {
                    self.pos += 1;
                    let args = self.args()?;
                    return Ok(Expr::Call(name, args));
                }
                Ok(Expr::Var(name))
            }
            Tok::Op("(") => {
                let e = self.expr()?;
                self.expect_op(")")?;
                Ok(e)
            }
            Tok::Op("[") => {
                let mut items = Vec::new();
                if self.eat_op("]") {
                    return Ok(Expr::List(items));
                }
                loop {
                    items.push(self.expr()?);
                    if self.eat_op("]") {
                        break;
                    }
                    self.expect_op(",")?;
                    if self.eat_op("]") {
                        break;
                    }
                }
                Ok(Expr::List(items))
            }
            Tok::Op(o) => Err(format!("operador inesperado '{}'", o)),
        }
    }
}

pub fn parse_expr_str(src: &str) -> Result<Expr, String> {
    let mut c = Cursor::new(src)?;
    let e = c.expr()?;
    c.expect_end()?;
    Ok(e)
}

/// Convierte el contenido de un texto "..." en una plantilla con `${expr}`.
pub fn parse_template(raw: &str) -> Result<Expr, String> {
    if !raw.contains("${") {
        return Ok(Expr::Lit(Value::Str(raw.to_string())));
    }
    let c: Vec<char> = raw.chars().collect();
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '\\' && c.get(i + 1) == Some(&'$') {
            text.push('$');
            i += 2;
            continue;
        }
        if c[i] == '$' && c.get(i + 1) == Some(&'{') {
            let end = lexer::skip_interp(&c, i + 2).ok_or("interpolación sin cerrar (falta })")?;
            let inner: String = c[i + 2..end].iter().collect();
            match parse_expr_str(&inner) {
                Ok(e) => {
                    if !text.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut text)));
                    }
                    parts.push(Part::Expr(e, inner));
                }
                // No es una expresión ACM (p. ej. código JavaScript): se deja literal.
                Err(_) => {
                    text.push_str("${");
                    text.push_str(&inner);
                    text.push('}');
                }
            }
            i = end + 1;
            continue;
        }
        text.push(c[i]);
        i += 1;
    }
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
    Ok(Expr::Tpl(parts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence() {
        let e = parse_expr_str("1 + 2 * 3 == 7 && !EMPTY x").unwrap();
        assert!(matches!(e, Expr::And(_, _)));
    }

    #[test]
    fn templates() {
        let e = parse_template("http://192.168.1.${i + 1}:${port}").unwrap();
        match e {
            Expr::Tpl(p) => assert_eq!(p.len(), 4),
            _ => panic!(),
        }
    }

    #[test]
    fn multiple_args() {
        let mut c = Cursor::new("100 200 WRITE \"hola\"").unwrap();
        c.expr().unwrap();
        c.expr().unwrap();
        assert!(c.eat_kw("WRITE"));
    }
}
