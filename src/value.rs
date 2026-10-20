use std::fmt;

/// Valor dinámico del lenguaje ACM.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    List(Vec<Value>),
}

impl Value {
    pub fn str(s: impl Into<String>) -> Self {
        Value::Str(s.into())
    }

    pub fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Num(n) => *n != 0.0 && !n.is_nan(),
            Value::Str(s) => !s.is_empty() && !s.eq_ignore_ascii_case("false"),
            Value::List(l) => !l.is_empty(),
        }
    }

    /// Interpreta el valor como número (los textos numéricos también cuentan).
    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            Value::Str(s) => parse_num(s),
            _ => None,
        }
    }

    pub fn is_empty_like(&self) -> bool {
        match self {
            Value::Null => true,
            Value::Str(s) => s.trim().is_empty(),
            Value::List(l) => l.is_empty(),
            _ => false,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Num(_) => "number",
            Value::Str(_) => "string",
            Value::List(_) => "list",
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    serde_json::json!(*n as i64)
                } else {
                    serde_json::json!(n)
                }
            }
            Value::Str(s) => serde_json::Value::String(s.clone()),
            Value::List(l) => serde_json::Value::Array(l.iter().map(|v| v.to_json()).collect()),
        }
    }

    pub fn from_json(v: &serde_json::Value) -> Value {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => Value::Num(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(s) => Value::Str(s.clone()),
            serde_json::Value::Array(a) => Value::List(a.iter().map(Value::from_json).collect()),
            other => Value::Str(other.to_string()),
        }
    }
}

pub fn parse_num(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() || !t.bytes().any(|b| b.is_ascii_digit()) {
        return None;
    }
    if !t
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'+' | b'e' | b'E'))
    {
        return None;
    }
    t.parse::<f64>().ok()
}

pub fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => Ok(()),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Num(n) => write!(f, "{}", fmt_num(*n)),
            Value::Str(s) => write!(f, "{}", s),
            Value::List(l) => {
                let parts: Vec<String> = l.iter().map(|v| v.to_string()).collect();
                write!(f, "{}", parts.join(", "))
            }
        }
    }
}
