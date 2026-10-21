//! Acceso a los archivos de la carpeta de trabajo.

use std::path::Path;

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    name: String,
    path: String,
    is_dir: bool,
}

const HIDDEN: &[&str] = &[".git", "node_modules", "target"];

/// Contenido de una carpeta (sin recursión): primero las carpetas y luego los archivos.
#[tauri::command]
pub fn fs_list(path: String) -> Result<Vec<Entry>, String> {
    let mut entries: Vec<Entry> = std::fs::read_dir(&path)
        .map_err(|e| format!("no se pudo leer '{}': {}", path, e))?
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if HIDDEN.contains(&name.as_str()) {
                return None;
            }
            let is_dir = e.file_type().ok()?.is_dir();
            Some(Entry { name, path: e.path().to_string_lossy().into(), is_dir })
        })
        .collect();
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(entries)
}

#[tauri::command]
pub fn fs_read(path: String) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("no se pudo abrir '{}': {}", path, e))?;
    Ok(String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}').to_string())
}

#[tauri::command]
pub fn fs_write(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| format!("no se pudo guardar '{}': {}", path, e))
}

/// Crea un archivo vacío o una carpeta. Falla si ya existe.
#[tauri::command]
pub fn fs_create(path: String, dir: bool) -> Result<(), String> {
    if Path::new(&path).exists() {
        return Err(format!("'{}' ya existe", path));
    }
    let r = if dir { std::fs::create_dir_all(&path) } else { std::fs::write(&path, "") };
    r.map_err(|e| format!("no se pudo crear '{}': {}", path, e))
}

#[tauri::command]
pub fn fs_rename(from: String, to: String) -> Result<(), String> {
    if Path::new(&to).exists() {
        return Err(format!("'{}' ya existe", to));
    }
    std::fs::rename(&from, &to).map_err(|e| format!("no se pudo renombrar '{}': {}", from, e))
}

#[tauri::command]
pub fn fs_delete(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    let r = if p.is_dir() { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) };
    r.map_err(|e| format!("no se pudo borrar '{}': {}", path, e))
}
