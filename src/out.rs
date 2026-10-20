//! Salida del intérprete: texto para personas o JSON por línea para agentes.

use std::io::IsTerminal;

use serde_json::json;

fn paint(code: &str, text: &str) -> String {
    if std::io::stderr().is_terminal() { format!("\x1b[{}m{}\x1b[0m", code, text) } else { text.to_string() }
}

#[derive(Clone, Copy, Default)]
pub struct Out {
    pub json: bool,
    pub verbose: bool,
}

impl Out {
    fn emit(&self, v: serde_json::Value) {
        println!("{}", v);
    }

    pub fn print(&self, text: &str) {
        if self.json {
            self.emit(json!({"event": "print", "text": text}));
        } else {
            println!("{}", text);
        }
    }

    pub fn step(&self, line: usize, code: &str) {
        if !self.verbose {
            return;
        }
        if self.json {
            self.emit(json!({"event": "step", "line": line, "code": code}));
        } else {
            eprintln!("{}", paint("2", &format!("[{:>4}] {}", line, code)));
        }
    }

    pub fn dry(&self, line: usize, action: &str) {
        if self.json {
            self.emit(json!({"event": "dry", "line": line, "action": action}));
        } else {
            println!("[dry-run] línea {}: {}", line, action);
        }
    }

    pub fn warn(&self, msg: &str) {
        if self.json {
            self.emit(json!({"event": "warning", "message": msg}));
        } else {
            eprintln!("{} {}", paint("33", "aviso:"), msg);
        }
    }

    pub fn error(&self, line: Option<usize>, msg: &str) {
        if self.json {
            self.emit(json!({"event": "error", "line": line, "message": msg}));
        } else {
            match line {
                Some(l) => eprintln!("{} (línea {}): {}", paint("31", "error"), l, msg),
                None => eprintln!("{}: {}", paint("31", "error"), msg),
            }
        }
    }

    pub fn event(&self, v: serde_json::Value) {
        if self.json {
            self.emit(v);
        }
    }
}
