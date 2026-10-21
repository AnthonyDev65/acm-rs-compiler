//! Puente con el intérprete `acm`. Todo pasa por su línea de comandos:
//! `-c --json` para comprobar la sintaxis, `--info --json` para leer los `@ARG`
//! y `--json` para ejecutar, con un evento JSON por línea de salida.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

const RUN_EVENT: &str = "acm://run";

#[derive(Default)]
pub struct Engine {
    path: Mutex<Option<PathBuf>>,
    next_id: AtomicU32,
    runs: Mutex<HashMap<u32, Run>>,
}

struct Run {
    pid: u32,
    stdin: Option<ChildStdin>,
}

#[derive(Serialize)]
pub struct EngineInfo {
    path: String,
    version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    ok: bool,
    line: Option<u32>,
    message: Option<String>,
    statements: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    /// Archivo guardado a ejecutar.
    path: Option<String>,
    /// Código sin guardar; se pasa por stdin (`acm -`).
    source: Option<String>,
    /// Código suelto de la consola; se pasa con `-e`, que muestra el valor de las expresiones.
    eval: Option<String>,
    cwd: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    trace: bool,
    #[serde(default)]
    keep_going: bool,
    #[serde(default)]
    no_dialogs: bool,
    #[serde(default)]
    no_failsafe: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RunMsg {
    run_id: u32,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    event: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<i32>,
}

impl RunMsg {
    fn new(run_id: u32, kind: &'static str) -> Self {
        RunMsg { run_id, kind, event: None, text: None, code: None }
    }
}

fn exe_name() -> String {
    format!("acm{}", std::env::consts::EXE_SUFFIX)
}

/// Rutas donde buscar `acm`, por orden de preferencia.
fn candidates() -> Vec<PathBuf> {
    let mut c = Vec::new();
    if let Some(p) = std::env::var_os("ACM_PATH") {
        c.push(PathBuf::from(p));
    }
    // Durante el desarrollo, el acm compilado en este mismo repositorio.
    if cfg!(debug_assertions) {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("target");
        c.push(repo.join("release").join(exe_name()));
        c.push(repo.join("debug").join(exe_name()));
    }
    if let Some(path) = std::env::var_os("PATH") {
        c.extend(std::env::split_paths(&path).map(|d| d.join(exe_name())));
    }
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        c.push(PathBuf::from(home).join(".cargo").join("bin").join(exe_name()));
    }
    c
}

/// `Command` para acm sin abrir una ventana de consola en Windows.
fn command(exe: &PathBuf) -> Command {
    let mut cmd = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn version_of(exe: &PathBuf) -> Option<String> {
    let out = command(exe).arg("--version").stdin(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

impl Engine {
    fn exe(&self) -> Result<PathBuf, String> {
        if let Some(p) = self.path.lock().unwrap().clone() {
            return Ok(p);
        }
        self.locate(None).map(|i| PathBuf::from(i.path))
    }

    fn locate(&self, custom: Option<String>) -> Result<EngineInfo, String> {
        let list = match custom.filter(|s| !s.trim().is_empty()) {
            Some(p) => vec![PathBuf::from(p.trim())],
            None => candidates(),
        };
        for p in list.into_iter().filter(|p| p.is_file()) {
            if let Some(version) = version_of(&p) {
                let path = p.canonicalize().unwrap_or(p);
                *self.path.lock().unwrap() = Some(path.clone());
                return Ok(EngineInfo { path: path.to_string_lossy().into(), version });
            }
        }
        *self.path.lock().unwrap() = None;
        Err("no se encontró el intérprete acm. Instálalo con `cargo install --path .` o indica su ruta en Ajustes.".into())
    }

    /// Ejecuta acm pasándole `source` por stdin y devuelve las líneas JSON de su salida.
    fn run_with_stdin(&self, args: &[&str], source: &str) -> Result<Vec<Value>, String> {
        let exe = self.exe()?;
        let mut child = command(&exe)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("no se pudo lanzar acm: {}", e))?;
        // Se escribe en otro hilo para no bloquearse si acm llena su salida antes de leer todo.
        let mut stdin = child.stdin.take().unwrap();
        let src = source.to_string();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(src.as_bytes());
        });
        let mut stdout = String::new();
        child.stdout.take().unwrap().read_to_string(&mut stdout).map_err(|e| e.to_string())?;
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        let _ = writer.join();
        let lines: Vec<Value> = stdout.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
        if lines.is_empty() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
        Ok(lines)
    }
}

#[tauri::command]
pub fn engine_locate(engine: State<Engine>, custom: Option<String>) -> Result<EngineInfo, String> {
    engine.locate(custom)
}

#[tauri::command]
pub async fn engine_check(engine: State<'_, Engine>, source: String) -> Result<CheckResult, String> {
    let lines = engine.run_with_stdin(&["-c", "--json", "-"], &source)?;
    for l in &lines {
        match l["event"].as_str() {
            Some("check") => {
                return Ok(CheckResult { ok: true, line: None, message: None, statements: l["statements"].as_u64().map(|n| n as u32) });
            }
            Some("error") => {
                return Ok(CheckResult {
                    ok: false,
                    line: l["line"].as_u64().map(|n| n as u32),
                    message: l["message"].as_str().map(String::from),
                    statements: None,
                });
            }
            _ => {}
        }
    }
    Err("respuesta inesperada de acm -c".into())
}

#[tauri::command]
pub async fn engine_info(engine: State<'_, Engine>, source: String) -> Result<Value, String> {
    let lines = engine.run_with_stdin(&["--info", "--json", "-"], &source)?;
    lines.into_iter().next().ok_or_else(|| "acm --info no devolvió nada".into())
}

#[tauri::command]
pub fn engine_run(app: AppHandle, engine: State<Engine>, req: RunRequest) -> Result<u32, String> {
    let exe = engine.exe()?;
    let mut cmd = command(&exe);
    cmd.arg("--json");
    for (on, flag) in [
        (req.dry_run, "--dry-run"),
        (req.trace, "--verbose"),
        (req.keep_going, "--keep-going"),
        (req.no_dialogs, "--no-dialogs"),
        (req.no_failsafe, "--no-failsafe"),
    ] {
        if on {
            cmd.arg(flag);
        }
    }
    let piped_source = if let Some(code) = &req.eval {
        cmd.arg("-e").arg(code);
        None
    } else if let Some(p) = &req.path {
        cmd.arg(p);
        None
    } else {
        cmd.arg("-");
        Some(req.source.clone().unwrap_or_default())
    };
    cmd.args(&req.args);
    if let Some(dir) = req.cwd.as_ref().filter(|d| !d.is_empty()) {
        cmd.current_dir(dir);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("no se pudo lanzar acm: {}", e))?;

    let id = engine.next_id.fetch_add(1, Ordering::SeqCst) + 1;
    let mut stdin = child.stdin.take();
    if let Some(src) = piped_source {
        // El script llega por stdin, así que no queda stdin para ASK.
        if let Some(mut s) = stdin.take() {
            std::thread::spawn(move || {
                let _ = s.write_all(src.as_bytes());
            });
        }
    }
    engine.runs.lock().unwrap().insert(id, Run { pid: child.id(), stdin });

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let app_err = app.clone();
    let err_thread = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = app_err.emit(RUN_EVENT, RunMsg { text: Some(line), ..RunMsg::new(id, "stderr") });
        }
    });
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let msg = match serde_json::from_str::<Value>(&line) {
                Ok(v) if v.get("event").is_some() => RunMsg { event: Some(v), ..RunMsg::new(id, "event") },
                _ => RunMsg { text: Some(line), ..RunMsg::new(id, "stdout") },
            };
            let _ = app.emit(RUN_EVENT, msg);
        }
        let _ = err_thread.join();
        let code = child.wait().ok().and_then(|s| s.code());
        if let Some(engine) = tauri::Manager::try_state::<Engine>(&app) {
            engine.runs.lock().unwrap().remove(&id);
        }
        let _ = app.emit(RUN_EVENT, RunMsg { code, ..RunMsg::new(id, "exit") });
    });
    Ok(id)
}

/// Detiene la ejecución y los procesos que haya abierto (por ejemplo, el navegador).
#[tauri::command]
pub fn engine_stop(engine: State<Engine>, run_id: u32) -> Result<(), String> {
    let pid = match engine.runs.lock().unwrap().get(&run_id) {
        Some(r) => r.pid,
        None => return Ok(()),
    };
    #[cfg(windows)]
    let st = {
        use std::os::windows::process::CommandExt;
        Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).creation_flags(0x0800_0000).output()
    };
    #[cfg(not(windows))]
    let st = Command::new("kill").args(["-9", &pid.to_string()]).output();
    st.map(|_| ()).map_err(|e| format!("no se pudo detener acm: {}", e))
}

/// Envía una línea a la entrada estándar de la ejecución (respuesta a ASK).
#[tauri::command]
pub fn engine_input(engine: State<Engine>, run_id: u32, text: String) -> Result<(), String> {
    let mut runs = engine.runs.lock().unwrap();
    let stdin = runs.get_mut(&run_id).and_then(|r| r.stdin.as_mut()).ok_or("la ejecución no admite entrada")?;
    stdin.write_all(format!("{}\n", text).as_bytes()).and_then(|_| stdin.flush()).map_err(|e| e.to_string())
}
