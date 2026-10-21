//! Cómo se abrió la aplicación y qué ventana mostrar:
//! - `acm-ide`                   → el IDE.
//! - `acm-ide archivo.acmx`      → la ventana de la extensión (formulario + ejecución).
//! - `acm-ide --edit archivo`    → el IDE con ese archivo abierto.
//! - `acm-ide --register`        → asocia los .acmx con esta aplicación y sale.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

pub struct Launch {
    /// Extensión que muestra la ventana de ejecución.
    runner_file: Option<String>,
    /// Archivo que el IDE debe abrir al arrancar o cuando lo pida la ventana de ejecución.
    pending_open: Mutex<Option<String>>,
}

pub enum Start {
    Window(Launch),
    /// Tarea de línea de comandos ya hecha; el código de salida.
    Exit(i32),
}

fn absolute(p: &str) -> String {
    let path = PathBuf::from(p);
    let full = if path.is_absolute() { path } else { std::env::current_dir().map(|d| d.join(&path)).unwrap_or(path) };
    full.to_string_lossy().into()
}

pub fn from_args() -> Start {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--register") {
        return Start::Exit(match associate() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{}", e);
                1
            }
        });
    }
    let edit = args.iter().position(|a| a == "--edit").and_then(|i| args.get(i + 1)).map(|p| absolute(p));
    let file = args.iter().find(|a| !a.starts_with("--") && Some(absolute(a)) != edit).map(|p| absolute(p));
    Start::Window(Launch { runner_file: if edit.is_some() { None } else { file }, pending_open: Mutex::new(edit) })
}

/// Crea la ventana inicial según el modo de arranque.
pub fn open_initial(app: &AppHandle) -> tauri::Result<()> {
    let runner = app.state::<Launch>().runner_file.is_some();
    if runner {
        WebviewWindowBuilder::new(app, "runner", WebviewUrl::App("index.html?runner".into()))
            .title("acm")
            .inner_size(500.0, 640.0)
            .min_inner_size(420.0, 420.0)
            .decorations(false)
            .shadow(true)
            .center()
            .build()?;
    } else {
        ide_window(app)?;
    }
    Ok(())
}

fn ide_window(app: &AppHandle) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("acm IDE")
        .inner_size(1280.0, 800.0)
        .min_inner_size(720.0, 480.0)
        .decorations(false)
        .shadow(true)
        .center()
        .build()?;
    Ok(())
}

#[derive(Serialize)]
pub struct LaunchInfo {
    file: Option<String>,
}

#[tauri::command]
pub fn launch_info(launch: State<Launch>) -> LaunchInfo {
    LaunchInfo { file: launch.runner_file.clone() }
}

/// El IDE pregunta al arrancar si tiene que abrir algún archivo.
#[tauri::command]
pub fn take_open_request(launch: State<Launch>) -> Option<String> {
    launch.pending_open.lock().unwrap().take()
}

/// Abre (o enfoca) el IDE con un archivo.
#[tauri::command]
pub fn open_in_ide(app: AppHandle, launch: State<Launch>, path: String) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
        return w.emit("ide://open", path).map_err(|e| e.to_string());
    }
    *launch.pending_open.lock().unwrap() = Some(path);
    ide_window(&app).map_err(|e| e.to_string())
}

// ---------- Asociación de archivos (Windows) ----------

#[cfg(windows)]
const PROG_ID: &str = "acm.Extension";

#[cfg(windows)]
fn reg(args: &[&str]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("reg").args(args).creation_flags(0x0800_0000).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(windows)]
fn associate() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?.to_string_lossy().to_string();
    let classes = r"HKCU\Software\Classes";
    let entries = [
        (format!(r"{}\.acmx", classes), PROG_ID.to_string()),
        (format!(r"{}\{}", classes, PROG_ID), "Extensión de acm".to_string()),
        (format!(r"{}\{}\DefaultIcon", classes, PROG_ID), format!("\"{}\",0", exe)),
        (format!(r"{}\{}\shell\open", classes, PROG_ID), "Ejecutar".to_string()),
        (format!(r"{}\{}\shell\open\command", classes, PROG_ID), format!("\"{}\" \"%1\"", exe)),
        (format!(r"{}\{}\shell\edit", classes, PROG_ID), "Editar en acm IDE".to_string()),
        (format!(r"{}\{}\shell\edit\command", classes, PROG_ID), format!("\"{}\" --edit \"%1\"", exe)),
    ];
    for (key, value) in entries {
        reg(&["add", &key, "/ve", "/d", &value, "/f"]).map_err(|e| format!("no se pudo escribir {}: {}", key, e))?;
    }
    // Una elección previa de "Abrir con" tendría prioridad sobre la asociación.
    let _ = reg(&["delete", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.acmx\UserChoice", "/f"]);
    Ok(())
}

#[cfg(not(windows))]
fn associate() -> Result<(), String> {
    Err("la asociación de archivos solo está disponible en Windows; usa el instalador".into())
}

/// Si los .acmx se abren con esta aplicación.
#[tauri::command]
pub fn acmx_associated() -> bool {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default();
        let ext = reg(&["query", r"HKCU\Software\Classes\.acmx", "/ve"]).unwrap_or_default();
        let cmd = reg(&["query", &format!(r"HKCU\Software\Classes\{}\shell\open\command", PROG_ID), "/ve"]).unwrap_or_default();
        ext.contains(PROG_ID) && cmd.to_lowercase().contains(&exe)
    }
    #[cfg(not(windows))]
    false
}

#[tauri::command]
pub fn associate_acmx() -> Result<(), String> {
    associate()
}
