//! Directivas de extensión (`@EXTENSION`, `@ARG`, ...) para archivos .acmx (y también .acm).

use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum ArgKind {
    Text,
    Check,
    Select(Vec<String>),
    List,
}

#[derive(Debug, Clone)]
pub struct ArgDef {
    pub name: String,
    pub label: String,
    pub kind: ArgKind,
    pub default: String,
}

#[derive(Debug, Default)]
pub struct Extension {
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub icon: Option<String>,
    pub args: Vec<ArgDef>,
}

/// Divide en partes respetando comillas: `name "Etiqueta" "valor"`.
fn quoted_parts(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    for ch in s.chars() {
        match ch {
            '"' => {
                if in_q {
                    parts.push(std::mem::take(&mut cur));
                }
                in_q = !in_q;
            }
            c if c.is_whitespace() && !in_q => {
                if !cur.is_empty() {
                    parts.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    parts
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    t.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(t).to_string()
}

pub fn parse(src: &str) -> Extension {
    let mut ext = Extension::default();
    for line in src.lines() {
        let t = line.trim().trim_start_matches('\u{feff}');
        if !t.starts_with('@') {
            continue;
        }
        let (dir, rest) = match t.find(char::is_whitespace) {
            Some(i) => (&t[..i], t[i..].trim()),
            None => (t, ""),
        };
        let up = dir.to_ascii_uppercase();
        match up.as_str() {
            "@EXTENSION" | "@NAME" => ext.name = Some(unquote(rest)),
            "@DESCRIPTION" => ext.description = Some(unquote(rest)),
            "@AUTHOR" => ext.author = Some(unquote(rest)),
            "@ICON" => ext.icon = Some(unquote(rest)),
            _ if up.starts_with("@ARG") => {
                let kind_word = up.strip_prefix("@ARG").unwrap().trim_start_matches(':');
                let parts = quoted_parts(rest);
                if parts.is_empty() {
                    continue;
                }
                let name = parts[0].clone();
                let label = parts.get(1).cloned().unwrap_or_else(|| name.clone());
                let (kind, default) = match kind_word {
                    "CHECK" | "BOOL" => {
                        let d = parts.get(2).map(|s| s.to_lowercase()).unwrap_or_default();
                        (ArgKind::Check, if ["true", "1", "si", "sí", "yes"].contains(&d.as_str()) { "true".into() } else { "false".into() })
                    }
                    "SELECT" => {
                        // 3 partes: "a|b|c" (por defecto la primera). 4 partes: "defecto" "a|b|c".
                        if parts.len() >= 4 {
                            let opts: Vec<String> = parts[3].split('|').map(|s| s.trim().to_string()).collect();
                            (ArgKind::Select(opts), parts[2].clone())
                        } else {
                            let opts: Vec<String> = parts.get(2).map(|p| p.split('|').map(|s| s.trim().to_string()).collect()).unwrap_or_default();
                            let d = opts.first().cloned().unwrap_or_default();
                            (ArgKind::Select(opts), d)
                        }
                    }
                    "LIST" => (ArgKind::List, parts.get(2).cloned().unwrap_or_default()),
                    _ => (ArgKind::Text, parts.get(2).cloned().unwrap_or_default()),
                };
                ext.args.push(ArgDef { name, label, kind, default });
            }
            _ => {}
        }
    }
    ext
}

/// Convierte el texto de un argumento en un valor según su tipo.
pub fn arg_value(def: Option<&ArgDef>, raw: &str) -> Result<Value, String> {
    let kind = def.map(|d| &d.kind).unwrap_or(&ArgKind::Text);
    match kind {
        ArgKind::Check => {
            let l = raw.trim().to_lowercase();
            Ok(Value::Bool(["true", "1", "si", "sí", "yes", "on"].contains(&l.as_str())))
        }
        ArgKind::Select(opts) => {
            if !opts.is_empty() && !opts.iter().any(|o| o == raw) {
                return Err(format!("valor '{}' no válido para '{}'; opciones: {}", raw, def.unwrap().name, opts.join(" | ")));
            }
            Ok(Value::str(raw))
        }
        ArgKind::List => Ok(Value::List(
            raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).map(Value::str).collect(),
        )),
        ArgKind::Text => Ok(Value::str(raw)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_args() {
        let ext = parse(
            r#"@EXTENSION "Lista"
@ARG base_ip "IP Base" "192.168.1"
@ARG:SELECT protocolo "Protocolo" "http|https|ftp"
@ARG:SELECT modo "Modo" "b" "a|b|c"
@ARG:CHECK puerto "Puerto" "false"
@ARG:LIST IPS "IPs" "1.1.1.1,2.2.2.2""#,
        );
        assert_eq!(ext.name.as_deref(), Some("Lista"));
        assert_eq!(ext.args.len(), 5);
        assert_eq!(ext.args[1].default, "http");
        assert_eq!(ext.args[2].default, "b");
        assert_eq!(ext.args[4].kind, ArgKind::List);
    }
}
