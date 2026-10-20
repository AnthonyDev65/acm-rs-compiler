//! Tokenizador de expresiones y utilidades de escaneo de texto.

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Num(f64),
    /// Contenido de "..." (con `${}` aún sin resolver).
    Str(String),
    /// Contenido de '...' (literal, sin interpolación).
    RawStr(String),
    Ident(String),
    /// `${...}` fuera de un texto: fuente de la expresión interior.
    Interp(String),
    Op(&'static str),
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub start: usize,
    pub end: usize,
}

const OPS: &[&str] = &[
    "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "++", "--", "->", "<", ">", "=",
    "+", "-", "*", "/", "%", "!", "(", ")", "[", "]", ",", ".", ":",
];

/// Dado el índice justo después de la comilla de apertura, devuelve el índice de la comilla de cierre.
pub fn skip_dq(c: &[char], mut i: usize) -> Option<usize> {
    while i < c.len() {
        match c[i] {
            '\\' => i += 2,
            '"' => return Some(i),
            '$' if c.get(i + 1) == Some(&'{') => i = skip_interp(c, i + 2)? + 1,
            _ => i += 1,
        }
    }
    None
}

pub fn skip_sq(c: &[char], mut i: usize) -> Option<usize> {
    while i < c.len() {
        match c[i] {
            '\\' => i += 2,
            '\'' => return Some(i),
            _ => i += 1,
        }
    }
    None
}

/// Dado el índice justo después de `${`, devuelve el índice de la `}` correspondiente.
pub fn skip_interp(c: &[char], mut i: usize) -> Option<usize> {
    let mut depth = 0usize;
    while i < c.len() {
        match c[i] {
            '"' => i = skip_dq(c, i + 1)? + 1,
            '\'' => i = skip_sq(c, i + 1)? + 1,
            '{' => {
                depth += 1;
                i += 1
            }
            '}' => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
                i += 1
            }
            _ => i += 1,
        }
    }
    None
}

pub fn is_ident_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

pub fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

pub fn lex(src: &str) -> Result<Vec<Token>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if ch == '"' {
            let end = skip_dq(&c, i + 1).ok_or("texto sin cerrar (falta \")")?;
            let raw: String = c[i + 1..end].iter().collect();
            out.push(Token { tok: Tok::Str(raw.replace("\\\"", "\"")), start, end: end + 1 });
            i = end + 1;
            continue;
        }
        if ch == '\'' {
            let end = skip_sq(&c, i + 1).ok_or("texto sin cerrar (falta ')")?;
            let raw: String = c[i + 1..end].iter().collect();
            out.push(Token { tok: Tok::RawStr(raw.replace("\\'", "'")), start, end: end + 1 });
            i = end + 1;
            continue;
        }
        if ch == '$' && c.get(i + 1) == Some(&'{') {
            let end = skip_interp(&c, i + 2).ok_or("interpolación sin cerrar (falta })")?;
            let inner: String = c[i + 2..end].iter().collect();
            out.push(Token { tok: Tok::Interp(inner), start, end: end + 1 });
            i = end + 1;
            continue;
        }
        if ch.is_ascii_digit() {
            let mut j = i;
            while j < c.len() && c[j].is_ascii_digit() {
                j += 1;
            }
            if j + 1 < c.len() && c[j] == '.' && c[j + 1].is_ascii_digit() {
                j += 1;
                while j < c.len() && c[j].is_ascii_digit() {
                    j += 1;
                }
            }
            let s: String = c[i..j].iter().collect();
            out.push(Token { tok: Tok::Num(s.parse().unwrap()), start, end: j });
            i = j;
            continue;
        }
        if is_ident_start(ch) {
            let mut j = i;
            while j < c.len() && is_ident_char(c[j]) {
                j += 1;
            }
            out.push(Token { tok: Tok::Ident(c[i..j].iter().collect()), start, end: j });
            i = j;
            continue;
        }
        let mut matched = false;
        for op in OPS {
            let oc: Vec<char> = op.chars().collect();
            if c[i..].starts_with(&oc) {
                out.push(Token { tok: Tok::Op(op), start, end: i + oc.len() });
                i += oc.len();
                matched = true;
                break;
            }
        }
        if !matched {
            return Err(format!("carácter inesperado '{}'", ch));
        }
    }
    Ok(out)
}

/// Divide una línea en sentencias por `;` (fuera de textos, paréntesis y corchetes)
/// y elimina comentarios `//` en línea.
pub fn split_statements(line: &str) -> Vec<String> {
    let c: Vec<char> = line.chars().collect();
    let mut parts = Vec::new();
    let mut cur_start = 0;
    let mut depth: i32 = 0;
    let mut i = 0;
    let mut end_of_code = c.len();
    while i < c.len() {
        match c[i] {
            '"' => match skip_dq(&c, i + 1) {
                Some(e) => {
                    i = e + 1;
                    continue;
                }
                None => break,
            },
            '\'' if i == 0 || !is_ident_char(c[i - 1]) => match skip_sq(&c, i + 1) {
                Some(e) => {
                    i = e + 1;
                    continue;
                }
                None => {
                    i += 1;
                    continue;
                }
            },
            '$' if c.get(i + 1) == Some(&'{') => match skip_interp(&c, i + 2) {
                Some(e) => {
                    i = e + 1;
                    continue;
                }
                None => break,
            },
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '/' if c.get(i + 1) == Some(&'/') && (i == 0 || c[i - 1].is_whitespace()) => {
                end_of_code = i;
                break;
            }
            ';' if depth <= 0 => {
                parts.push(c[cur_start..i].iter().collect::<String>());
                cur_start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if cur_start < end_of_code {
        parts.push(c[cur_start..end_of_code].iter().collect::<String>());
    }
    parts.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_comments() {
        assert_eq!(split_statements(r#"VAR X = "t"; WRITE X; KEYPRESS ENTER"#).len(), 3);
        assert_eq!(split_statements("FOR (VAR i = 0; i < 3; i++)").len(), 1);
        assert_eq!(split_statements("WEBTAB 0  // volver").as_slice(), ["WEBTAB 0"]);
        assert_eq!(split_statements(r#"WEBOPEN "http://x.com""#).len(), 1);
        assert_eq!(split_statements(r#"WRITE "${RANDOM("a","b")}"; KEY ENTER"#).len(), 2);
    }

    #[test]
    fn lexes() {
        let t = lex(r#"x == "a${b}" && y >= 2.5"#).unwrap();
        assert_eq!(t.len(), 7);
    }
}
