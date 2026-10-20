mod expr;
mod extension;
mod host;
mod interp;
mod lexer;
mod out;
mod parser;
mod value;
mod web;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use clap::Parser as ClapParser;
use serde_json::json;

use interp::{ErrKind, Interp, Options};
use out::Out;
use value::Value;

const EXAMPLES: &str = "\
Ejemplos:
  acm                              Consola interactiva (REPL)
  acm script.acm                   Ejecuta un script
  acm lista_ips.acmx base_ip=10.0.0 start=1 end=20
  acm -e 'CLICK 500 300; WRITE \"hola\"; KEYPRESS ENTER'
  acm -e 'PRINT mousex(), mousey()'
  acm script.acm --dry-run -v      Muestra lo que haría sin mover nada
  acm script.acm --json            Eventos JSON por línea (para agentes)
  echo 'PRINT 1 + 2' | acm -       Lee el script desde stdin
  acm --info extension.acmx        Muestra los argumentos de una extensión";

#[derive(ClapParser)]
#[command(name = "acm", version, about = "acm: automatiza tu PC (ratón, teclado, ventanas y web) con scripts .acm", after_help = EXAMPLES)]
struct Cli {
    /// Script .acm/.acmx a ejecutar ("-" para leer de stdin)
    file: Option<String>,

    /// Argumentos del script en formato nombre=valor
    args: Vec<String>,

    /// Ejecuta el código indicado en lugar de un archivo
    #[arg(short = 'e', long = "eval", value_name = "CÓDIGO")]
    eval: Option<String>,

    /// Solo comprueba la sintaxis, sin ejecutar
    #[arg(short = 'c', long)]
    check: bool,

    /// Muestra los metadatos y argumentos de una extensión
    #[arg(long)]
    info: bool,

    /// Salida como eventos JSON (uno por línea)
    #[arg(long)]
    json: bool,

    /// Muestra cada sentencia antes de ejecutarla
    #[arg(short, long)]
    verbose: bool,

    /// Simula: no mueve el ratón, no teclea, no abre nada
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Continúa aunque un comando falle
    #[arg(short = 'k', long = "keep-going")]
    keep_going: bool,

    /// ALERT y NOTIFY se escriben en la consola en vez de mostrar ventanas
    #[arg(long = "no-dialogs")]
    no_dialogs: bool,

    /// Desactiva la parada de emergencia (ratón en la esquina superior izquierda)
    #[arg(long = "no-failsafe")]
    no_failsafe: bool,

    /// Pregunta por consola los argumentos @ARG que no se hayan pasado
    #[arg(long)]
    ask: bool,

    /// Abre la consola interactiva después de ejecutar el script
    #[arg(short = 'i', long)]
    interactive: bool,

    /// Asocia los archivos .acm/.acmx con acm (Windows)
    #[arg(long)]
    register: bool,
}

fn main() {
    let mut cli = match windows_eval_argv() {
        Some(argv) => Cli::parse_from(argv),
        None => Cli::parse(),
    };
    enable_ansi();
    host::init_platform();

    let cancel = Arc::new(AtomicBool::new(false));
    {
        let c = cancel.clone();
        let _ = ctrlc::set_handler(move || {
            if c.swap(true, Ordering::SeqCst) {
                std::process::exit(130);
            }
        });
    }

    if cli.register {
        std::process::exit(register());
    }

    let out = Out { json: cli.json, verbose: cli.verbose };
    let opts = Options {
        dry_run: cli.dry_run,
        continue_on_error: cli.keep_going,
        failsafe: !cli.no_failsafe,
        dialogs: !cli.no_dialogs,
        echo: false,
    };

    let code = if let Some(code) = cli.eval.take() {
        if let Some(f) = cli.file.take() {
            cli.args.insert(0, f);
        }
        let mut o = opts.clone();
        o.echo = true;
        run_source(&cli, &code, "<-e>", None, o, out, cancel)
    } else if let Some(file) = cli.file.clone() {
        let (src, path) = if file == "-" {
            let mut s = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut s) {
                out.error(None, &format!("no se pudo leer stdin: {}", e));
                std::process::exit(2);
            }
            (s, None)
        } else {
            let p = PathBuf::from(&file);
            match std::fs::read(&p) {
                Ok(bytes) => (String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}').to_string(), Some(p)),
                Err(e) => {
                    out.error(None, &format!("no se pudo abrir '{}': {}", file, e));
                    std::process::exit(2);
                }
            }
        };
        run_source(&cli, &src, &file, path.as_deref(), opts, out, cancel)
    } else {
        let mut o = opts.clone();
        o.echo = true;
        let it = Interp::new(o, out, cancel.clone());
        repl(it, out, cancel)
    };
    std::process::exit(code);
}

fn run_source(cli: &Cli, src: &str, name: &str, path: Option<&Path>, opts: Options, out: Out, cancel: Arc<AtomicBool>) -> i32 {
    let ext = extension::parse(src);
    if cli.info {
        print_info(&ext, name, cli.json);
        return 0;
    }
    let prog = match parser::parse_program(src) {
        Ok(p) => p,
        Err(pe) => {
            out.error(Some(pe.line), &format!("error de sintaxis: {}", pe.msg));
            out.event(json!({"event": "done", "ok": false, "code": 2}));
            return 2;
        }
    };
    if cli.check {
        if cli.json {
            println!("{}", json!({"event": "check", "ok": true, "statements": prog.len()}));
        } else {
            println!("OK: {} — sintaxis correcta ({} sentencias)", name, prog.len());
        }
        return 0;
    }

    let mut it = Interp::new(opts, out, cancel.clone());
    if let Some(p) = path {
        if let Some(dir) = p.parent() {
            if !dir.as_os_str().is_empty() {
                it.base_dir = dir.to_path_buf();
            }
        }
        it.globals.insert("SCRIPT".into(), Value::str(p.to_string_lossy()));
    }
    if let Err(msg) = apply_args(&mut it, &ext, &cli.args, cli.ask) {
        out.error(None, &msg);
        return 2;
    }

    out.event(json!({"event": "start", "script": name, "statements": prog.len()}));
    let t0 = Instant::now();
    let code = match it.exec_block(&prog) {
        Ok(_) => 0,
        Err(e) => match e.kind {
            ErrKind::Exit(c) => c,
            ErrKind::Cancelled => {
                out.error(e.line, &e.msg);
                130
            }
            ErrKind::Runtime => {
                out.error(e.line, &e.msg);
                1
            }
        },
    };
    out.event(json!({"event": "done", "ok": code == 0, "code": code, "ms": t0.elapsed().as_millis() as u64}));

    if cli.interactive {
        cancel.store(false, Ordering::SeqCst);
        it.opts.echo = true;
        return repl(it, out, cancel);
    }
    code
}

fn apply_args(it: &mut Interp, ext: &extension::Extension, args: &[String], ask: bool) -> Result<(), String> {
    let mut given = std::collections::HashSet::new();
    let mut positional = Vec::new();
    let mut pairs = Vec::new();
    for a in args {
        match a.split_once('=') {
            Some((k, v)) if !k.is_empty() && k.chars().all(lexer::is_ident_char) => {
                given.insert(k.to_string());
                pairs.push((k.to_string(), v.to_string()));
            }
            _ => positional.push(Value::str(a)),
        }
    }
    for def in &ext.args {
        let mut raw = def.default.clone();
        if ask && !given.contains(&def.name) {
            let opts = match &def.kind {
                extension::ArgKind::Select(o) => format!(" ({})", o.join("|")),
                extension::ArgKind::Check => " (true/false)".into(),
                extension::ArgKind::List => " (separados por comas)".into(),
                _ => String::new(),
            };
            eprint!("{}{} [{}]: ", def.label, opts, def.default);
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).map_err(|e| e.to_string())?;
            let l = line.trim();
            if !l.is_empty() {
                raw = l.to_string();
            }
        }
        it.globals.insert(def.name.clone(), extension::arg_value(Some(def), &raw)?);
    }
    for (k, v) in pairs {
        let def = ext.args.iter().find(|d| d.name == k).or_else(|| ext.args.iter().find(|d| d.name.eq_ignore_ascii_case(&k)));
        let name = def.map(|d| d.name.clone()).unwrap_or(k);
        it.globals.insert(name, extension::arg_value(def, &v)?);
    }
    it.globals.insert("ARGS".into(), Value::List(positional));
    Ok(())
}

fn print_info(ext: &extension::Extension, name: &str, as_json: bool) {
    if as_json {
        let args: Vec<_> = ext
            .args
            .iter()
            .map(|a| {
                let (kind, options) = match &a.kind {
                    extension::ArgKind::Text => ("text", vec![]),
                    extension::ArgKind::Check => ("check", vec![]),
                    extension::ArgKind::List => ("list", vec![]),
                    extension::ArgKind::Select(o) => ("select", o.clone()),
                };
                json!({"name": a.name, "label": a.label, "type": kind, "default": a.default, "options": options})
            })
            .collect();
        println!(
            "{}",
            json!({"file": name, "name": ext.name, "description": ext.description, "author": ext.author, "icon": ext.icon, "args": args})
        );
        return;
    }
    println!("{} {}", ext.icon.clone().unwrap_or_else(|| "🧩".into()), ext.name.clone().unwrap_or_else(|| name.to_string()));
    if let Some(d) = &ext.description {
        println!("  {}", d);
    }
    if let Some(a) = &ext.author {
        println!("  Autor: {}", a);
    }
    if ext.args.is_empty() {
        println!("\n  (sin argumentos)");
        return;
    }
    println!("\nArgumentos (nombre=valor):");
    for a in &ext.args {
        let kind = match &a.kind {
            extension::ArgKind::Text => "texto".to_string(),
            extension::ArgKind::Check => "true/false".to_string(),
            extension::ArgKind::List => "lista a,b,c".to_string(),
            extension::ArgKind::Select(o) => o.join(" | "),
        };
        println!("  {:<18} {:<28} [{}]  por defecto: {}", a.name, a.label, kind, a.default);
    }
}

const REPL_HELP: &str = "\
Escribe cualquier sentencia ACM; las expresiones muestran su valor.
Los bloques (IF, FOR, WHILE, LOOP, FUNC) continúan hasta su cierre.

Meta-comandos:
  .help            esta ayuda
  .vars            variables definidas
  .funcs           funciones definidas
  .load archivo    ejecuta un archivo en esta sesión
  .exit            salir (también Ctrl+D)

Ejemplos:
  PRINT mousex(), mousey()
  CLICK 200 300; WRITE \"hola\"
  FOR i = 1 TO 3
    PRINT \"vuelta ${i}\"
  NEXT
  windows()";

fn history_path() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(|h| PathBuf::from(h).join(".acm_history"))
}

fn repl(mut it: Interp, out: Out, cancel: Arc<AtomicBool>) -> i32 {
    use rustyline::error::ReadlineError;
    println!("acm {} — consola interactiva ({}). Escribe .help para ver la ayuda y .exit para salir.", env!("CARGO_PKG_VERSION"), std::env::consts::OS);
    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(r) => r,
        Err(e) => {
            out.error(None, &format!("no se pudo iniciar la consola: {}", e));
            return 1;
        }
    };
    let hist = history_path();
    if let Some(h) = &hist {
        let _ = rl.load_history(h);
    }
    let mut buf = String::new();
    let mut code = 0;
    loop {
        let prompt = if buf.is_empty() { "acm> " } else { "...> " };
        match rl.readline(prompt) {
            Ok(line) => {
                let t = line.trim();
                if buf.is_empty() {
                    if t.is_empty() {
                        continue;
                    }
                    if t.starts_with('.') {
                        let _ = rl.add_history_entry(t);
                        let (cmd, arg) = t.split_once(char::is_whitespace).unwrap_or((t, ""));
                        match cmd {
                            ".exit" | ".quit" | ".salir" => break,
                            ".help" | ".ayuda" => println!("{}", REPL_HELP),
                            ".vars" => {
                                let mut keys: Vec<_> = it.globals.keys().cloned().collect();
                                keys.sort();
                                for k in keys {
                                    let v = &it.globals[&k];
                                    println!("  {} = {} ({})", k, v.to_json(), v.type_name());
                                }
                            }
                            ".funcs" => {
                                let mut f = it.function_names();
                                f.sort();
                                println!("  {}", if f.is_empty() { "(ninguna)".to_string() } else { f.join(", ") });
                            }
                            ".load" => match std::fs::read_to_string(arg.trim().trim_matches('"')) {
                                Ok(src) => match parser::parse_program(&src) {
                                    Ok(prog) => {
                                        cancel.store(false, Ordering::SeqCst);
                                        if let Err(e) = it.exec_block(&prog) {
                                            out.error(e.line, &e.msg);
                                        }
                                    }
                                    Err(pe) => out.error(Some(pe.line), &format!("error de sintaxis: {}", pe.msg)),
                                },
                                Err(e) => out.error(None, &format!("no se pudo leer '{}': {}", arg.trim(), e)),
                            },
                            other => out.error(None, &format!("meta-comando desconocido '{}' (usa .help)", other)),
                        }
                        continue;
                    }
                    if t.eq_ignore_ascii_case("exit") || t.eq_ignore_ascii_case("quit") {
                        break;
                    }
                }
                buf.push_str(&line);
                buf.push('\n');
                match parser::parse_program(&buf) {
                    Err(pe) if pe.incomplete => continue,
                    Err(pe) => {
                        let _ = rl.add_history_entry(buf.trim_end());
                        out.error(None, &format!("error de sintaxis: {}", pe.msg));
                        buf.clear();
                    }
                    Ok(prog) => {
                        let _ = rl.add_history_entry(buf.trim_end());
                        buf.clear();
                        cancel.store(false, Ordering::SeqCst);
                        if let Err(e) = it.exec_block(&prog) {
                            match e.kind {
                                ErrKind::Exit(c) => {
                                    code = c;
                                    break;
                                }
                                _ => out.error(None, &e.msg),
                            }
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                if buf.is_empty() {
                    println!("(usa .exit o Ctrl+D para salir)");
                }
                buf.clear();
            }
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                out.error(None, &e.to_string());
                code = 1;
                break;
            }
        }
    }
    if let Some(h) = &hist {
        let _ = rl.save_history(h);
    }
    code
}

/// En Windows cada programa recibe la línea de comandos en bruto y la trocea a su manera:
/// cmd.exe no entiende las comillas simples y PowerShell 5.1 no escapa las comillas dobles
/// interiores. Para `-e`, se recupera el código exacto de la línea original.
#[cfg(windows)]
fn windows_eval_argv() -> Option<Vec<String>> {
    use windows_sys::Win32::System::Environment::GetCommandLineW;
    let raw = unsafe {
        let p = GetCommandLineW();
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    };
    let (code, leftovers) = split_raw_eval(&raw)?;
    let args: Vec<String> = std::env::args().collect();
    let pos = args.iter().position(|a| a == "-e" || a == "--eval")?;
    let mut argv: Vec<String> = args[..=pos].to_vec();
    argv.push(code);
    argv.extend(leftovers);
    Some(argv)
}

#[cfg(not(windows))]
fn windows_eval_argv() -> Option<Vec<String>> {
    None
}

/// Separa el código que sigue a `-e` (entre comillas simples o dobles) del resto de argumentos.
#[cfg_attr(not(windows), allow(dead_code))]
fn split_raw_eval(raw: &str) -> Option<(String, Vec<String>)> {
    let c: Vec<char> = raw.chars().collect();
    let mut i = 0;
    let mut rest_start = None;
    while i < c.len() {
        while i < c.len() && c[i].is_whitespace() {
            i += 1;
        }
        let s = i;
        let mut in_q = false;
        while i < c.len() && (in_q || !c[i].is_whitespace()) {
            if c[i] == '"' {
                in_q = !in_q;
            }
            i += 1;
        }
        let tok: String = c[s..i].iter().collect();
        if tok == "-e" || tok == "--eval" {
            rest_start = Some(i);
            break;
        }
    }
    let rest: Vec<char> = c[rest_start?..].iter().copied().skip_while(|ch| ch.is_whitespace()).collect();
    let q = *rest.first()?;
    if q != '\'' && q != '"' {
        return None;
    }
    // La comilla de cierre es la última igual a la de apertura seguida de espacio o fin de línea.
    let close = (1..rest.len()).rev().find(|&k| rest[k] == q && rest.get(k + 1).is_none_or(|ch| ch.is_whitespace()))?;
    let mut code: String = rest[1..close].iter().collect();
    if q == '"' {
        code = code.replace("\\\"", "\"");
    }
    let mut leftovers = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    for ch in rest[close + 1..].iter().copied() {
        match ch {
            '"' => in_q = !in_q,
            ch if ch.is_whitespace() && !in_q => {
                if !cur.is_empty() {
                    leftovers.push(std::mem::take(&mut cur));
                }
            }
            ch => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        leftovers.push(cur);
    }
    Some((code, leftovers))
}

#[cfg(test)]
mod tests {
    use super::split_raw_eval;

    #[test]
    fn raw_eval_lines() {
        // cmd.exe con comillas simples
        let (c, l) = split_raw_eval(r#"acm -e 'CLICK 500 300; WRITE "hola"' x=1 --json"#).unwrap();
        assert_eq!(c, r#"CLICK 500 300; WRITE "hola""#);
        assert_eq!(l, vec!["x=1", "--json"]);
        // PowerShell 5.1 (comillas interiores sin escapar)
        let (c, _) = split_raw_eval(r#""C:\x\acm.exe" -e "PRINT trim(exec("ver"))""#).unwrap();
        assert_eq!(c, r#"PRINT trim(exec("ver"))"#);
        // cmd con comillas escapadas correctamente
        let (c, _) = split_raw_eval(r#"acm --json -e "WRITE \"hola\"""#).unwrap();
        assert_eq!(c, r#"WRITE "hola""#);
        // sin comillas: se usa el troceado normal
        assert!(split_raw_eval("acm -e PRINT").is_none());
        assert!(split_raw_eval("acm script.acm").is_none());
    }
}

#[cfg(windows)]
fn enable_ansi() {
    use windows_sys::Win32::System::Console::{GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};
    unsafe {
        for h in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let handle = GetStdHandle(h);
            let mut mode = 0;
            if GetConsoleMode(handle, &mut mode) != 0 {
                SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            }
        }
    }
}

#[cfg(not(windows))]
fn enable_ansi() {}

#[cfg(windows)]
fn register() -> i32 {
    use std::process::Command;
    let exe = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(e) => {
            eprintln!("no se pudo obtener la ruta de acm: {}", e);
            return 1;
        }
    };
    let open_cmd = format!("\"{}\" \"%1\" %*", exe);
    let entries: Vec<(&str, String)> = vec![
        (r"HKCU\Software\Classes\.acm", "acm.Script".into()),
        (r"HKCU\Software\Classes\.acmx", "acm.Script".into()),
        (r"HKCU\Software\Classes\acm.Script", "Script de acm".into()),
        (r"HKCU\Software\Classes\acm.Script\shell\open\command", open_cmd),
    ];
    for (key, val) in entries {
        let st = Command::new("reg").args(["add", key, "/ve", "/d", &val, "/f"]).output();
        match st {
            Ok(o) if o.status.success() => {}
            _ => {
                eprintln!("no se pudo escribir {}", key);
                return 1;
            }
        }
    }
    let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    let up = pathext.to_ascii_uppercase();
    if !up.split(';').any(|e| e == ".ACM") {
        let new = format!("{};.ACM;.ACMX", pathext.trim_end_matches(';'));
        let _ = Command::new("setx").args(["PATHEXT", &new]).output();
    }
    println!("Listo: los archivos .acm y .acmx se abren con {}", exe);
    println!("Abre una terminal nueva y ya puedes ejecutar 'script.acm' directamente.");
    0
}

#[cfg(not(windows))]
fn register() -> i32 {
    println!("En macOS/Linux añade esta primera línea al script y dale permisos de ejecución:");
    println!("  #!/usr/bin/env acm");
    println!("  chmod +x script.acm && ./script.acm");
    0
}
