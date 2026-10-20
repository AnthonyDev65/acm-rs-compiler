//! Acceso al sistema: ratón, teclado, portapapeles, pantalla, ventanas, procesos y diálogos.

use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

use crate::parser::{Btn, KeyMode};

fn e<E: std::fmt::Display>(x: E) -> String {
    x.to_string()
}

pub struct Host {
    enigo: Option<Enigo>,
    clip: Option<arboard::Clipboard>,
}

impl Host {
    pub fn new() -> Self {
        Host { enigo: None, clip: None }
    }

    fn input(&mut self) -> Result<&mut Enigo, String> {
        if self.enigo.is_none() {
            let en = Enigo::new(&Settings::default()).map_err(|x| format!("no se pudo iniciar la simulación de entrada: {}", x))?;
            self.enigo = Some(en);
        }
        Ok(self.enigo.as_mut().unwrap())
    }

    fn clipboard(&mut self) -> Result<&mut arboard::Clipboard, String> {
        if self.clip.is_none() {
            self.clip = Some(arboard::Clipboard::new().map_err(|x| format!("portapapeles no disponible: {}", x))?);
        }
        Ok(self.clip.as_mut().unwrap())
    }

    // ---------- Ratón ----------

    pub fn mouse_pos(&mut self) -> Result<(i32, i32), String> {
        self.input()?.location().map_err(e)
    }

    pub fn screen_size(&mut self) -> Result<(i32, i32), String> {
        self.input()?.main_display().map_err(e)
    }

    pub fn move_to(&mut self, x: i32, y: i32) -> Result<(), String> {
        self.input()?.move_mouse(x, y, Coordinate::Abs).map_err(e)
    }

    pub fn click(&mut self, button: Btn, count: u8, pos: Option<(i32, i32)>) -> Result<(), String> {
        let en = self.input()?;
        if let Some((x, y)) = pos {
            en.move_mouse(x, y, Coordinate::Abs).map_err(e)?;
            sleep(Duration::from_millis(30));
        }
        let b = to_button(button);
        for i in 0..count {
            if i > 0 {
                sleep(Duration::from_millis(60));
            }
            en.button(b, Direction::Press).map_err(e)?;
            sleep(Duration::from_millis(30));
            en.button(b, Direction::Release).map_err(e)?;
        }
        Ok(())
    }

    pub fn mouse_button(&mut self, button: Btn, down: bool) -> Result<(), String> {
        let dir = if down { Direction::Press } else { Direction::Release };
        self.input()?.button(to_button(button), dir).map_err(e)
    }

    pub fn drag(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) -> Result<(), String> {
        let en = self.input()?;
        en.move_mouse(x1, y1, Coordinate::Abs).map_err(e)?;
        sleep(Duration::from_millis(30));
        en.button(Button::Left, Direction::Press).map_err(e)?;
        sleep(Duration::from_millis(50));
        let steps = 15;
        for s in 1..=steps {
            let x = x1 + (x2 - x1) * s / steps;
            let y = y1 + (y2 - y1) * s / steps;
            en.move_mouse(x, y, Coordinate::Abs).map_err(e)?;
            sleep(Duration::from_millis(12));
        }
        sleep(Duration::from_millis(30));
        en.button(Button::Left, Direction::Release).map_err(e)
    }

    pub fn scroll(&mut self, amount: i32, horizontal: bool) -> Result<(), String> {
        let axis = if horizontal { Axis::Horizontal } else { Axis::Vertical };
        self.input()?.scroll(amount, axis).map_err(e)
    }

    // ---------- Teclado ----------

    pub fn keys(&mut self, keys: &[Key], mode: KeyMode) -> Result<(), String> {
        let en = self.input()?;
        match mode {
            KeyMode::Down => {
                for k in keys {
                    en.key(*k, Direction::Press).map_err(e)?;
                }
            }
            KeyMode::Up => {
                for k in keys.iter().rev() {
                    en.key(*k, Direction::Release).map_err(e)?;
                }
            }
            KeyMode::Tap => {
                if keys.len() == 1 {
                    en.key(keys[0], Direction::Click).map_err(e)?;
                } else {
                    for k in keys {
                        en.key(*k, Direction::Press).map_err(e)?;
                        sleep(Duration::from_millis(15));
                    }
                    for k in keys.iter().rev() {
                        en.key(*k, Direction::Release).map_err(e)?;
                        sleep(Duration::from_millis(10));
                    }
                }
            }
        }
        Ok(())
    }

    /// Escribe un carácter (con la distribución de teclado actual).
    pub fn type_char(&mut self, ch: char) -> Result<(), String> {
        let en = self.input()?;
        match ch {
            '\n' => en.key(Key::Return, Direction::Click).map_err(e),
            '\t' => en.key(Key::Tab, Direction::Click).map_err(e),
            '\r' => Ok(()),
            _ => en.text(&ch.to_string()).map_err(e),
        }
    }

    pub fn paste(&mut self, text: &str) -> Result<(), String> {
        let previous = self.clipboard()?.get_text().ok();
        self.clipboard()?.set_text(text.to_string()).map_err(e)?;
        sleep(Duration::from_millis(40));
        self.keys(&[paste_modifier(), Key::Unicode('v')], KeyMode::Tap)?;
        sleep(Duration::from_millis(80));
        if let Some(prev) = previous {
            let _ = self.clipboard()?.set_text(prev);
        }
        Ok(())
    }

    pub fn clip_get(&mut self) -> Result<String, String> {
        Ok(self.clipboard()?.get_text().unwrap_or_default())
    }

    pub fn clip_set(&mut self, text: &str) -> Result<(), String> {
        self.clipboard()?.set_text(text.to_string()).map_err(e)
    }
}

fn to_button(b: Btn) -> Button {
    match b {
        Btn::Left => Button::Left,
        Btn::Right => Button::Right,
        Btn::Middle => Button::Middle,
    }
}

fn paste_modifier() -> Key {
    if cfg!(target_os = "macos") { Key::Meta } else { Key::Control }
}

/// Convierte un nombre de tecla (ENTER, CTRL, F5, a, ...) en una tecla de enigo.
pub fn key_from_name(name: &str) -> Option<Key> {
    let up = name.trim().to_ascii_uppercase();
    let k = match up.as_str() {
        "ENTER" | "RETURN" | "INTRO" => Key::Return,
        "SPACE" | "ESPACIO" => Key::Space,
        "TAB" => Key::Tab,
        "ESC" | "ESCAPE" => Key::Escape,
        "BACKSPACE" | "BACK" | "BORRAR" => Key::Backspace,
        "DELETE" | "DEL" | "SUPR" => Key::Delete,
        "HOME" | "INICIO" => Key::Home,
        "END" | "FIN" => Key::End,
        "PAGEUP" | "PGUP" => Key::PageUp,
        "PAGEDOWN" | "PGDN" => Key::PageDown,
        "LEFT" | "IZQUIERDA" => Key::LeftArrow,
        "RIGHT" | "DERECHA" => Key::RightArrow,
        "UP" | "ARRIBA" => Key::UpArrow,
        "DOWN" | "ABAJO" => Key::DownArrow,
        "SHIFT" | "MAYUS" | "LSHIFT" => Key::Shift,
        "RSHIFT" => Key::RShift,
        "CTRL" | "CONTROL" | "LCTRL" => Key::Control,
        "RCTRL" => Key::RControl,
        "ALT" | "OPTION" => Key::Alt,
        "WIN" | "WINDOWS" | "LWIN" | "META" | "SUPER" | "CMD" | "COMMAND" => Key::Meta,
        // Modificador principal de la plataforma: CMD en macOS, CTRL en el resto.
        "MOD" | "CMDORCTRL" => paste_modifier(),
        "CAPSLOCK" | "BLOQMAYUS" => Key::CapsLock,
        "VOLUMEUP" => Key::VolumeUp,
        "VOLUMEDOWN" => Key::VolumeDown,
        "VOLUMEMUTE" | "MUTE" => Key::VolumeMute,
        "PLAYPAUSE" => Key::MediaPlayPause,
        "NEXTTRACK" => Key::MediaNextTrack,
        "PREVTRACK" => Key::MediaPrevTrack,
        "PLUS" | "MAS" => Key::Unicode('+'),
        "MINUS" => Key::Unicode('-'),
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "NUMPAD0" => Key::Numpad0,
        "NUMPAD1" => Key::Numpad1,
        "NUMPAD2" => Key::Numpad2,
        "NUMPAD3" => Key::Numpad3,
        "NUMPAD4" => Key::Numpad4,
        "NUMPAD5" => Key::Numpad5,
        "NUMPAD6" => Key::Numpad6,
        "NUMPAD7" => Key::Numpad7,
        "NUMPAD8" => Key::Numpad8,
        "NUMPAD9" => Key::Numpad9,
        _ => return platform_key(&up).or_else(|| single_char(name.trim())),
    };
    Some(k)
}

#[cfg(target_os = "windows")]
fn platform_key(up: &str) -> Option<Key> {
    Some(match up {
        "INSERT" | "INS" => Key::Insert,
        "NUMLOCK" => Key::Numlock,
        "PAUSE" => Key::Pause,
        "PRINTSCREEN" | "PRTSC" | "IMPRPANT" => Key::PrintScr,
        "SCROLLLOCK" => Key::Other(0x91),
        "RWIN" => Key::Other(0x5C),
        "APPS" | "MENU" => Key::Apps,
        _ => return None,
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_key(up: &str) -> Option<Key> {
    Some(match up {
        "INSERT" | "INS" => Key::Insert,
        "NUMLOCK" => Key::Numlock,
        "PAUSE" => Key::Pause,
        "PRINTSCREEN" | "PRTSC" => Key::PrintScr,
        "SCROLLLOCK" => Key::ScrollLock,
        _ => return None,
    })
}

#[cfg(target_os = "macos")]
fn platform_key(_up: &str) -> Option<Key> {
    None
}

fn single_char(s: &str) -> Option<Key> {
    let mut it = s.chars();
    let c = it.next()?;
    if it.next().is_some() {
        return None;
    }
    Some(Key::Unicode(c.to_ascii_lowercase()))
}

// ---------- Pantalla ----------

fn primary_monitor() -> Result<xcap::Monitor, String> {
    let monitors = xcap::Monitor::all().map_err(e)?;
    let mut first = None;
    for m in monitors {
        if m.is_primary().unwrap_or(false) {
            return Ok(m);
        }
        if first.is_none() {
            first = Some(m);
        }
    }
    first.ok_or_else(|| "no se encontró ningún monitor".to_string())
}

pub fn screenshot(path: &str, region: Option<(i32, i32, u32, u32)>) -> Result<(), String> {
    let img = match region {
        None => primary_monitor()?.capture_image().map_err(e)?,
        Some((x, y, w, h)) => {
            let m = xcap::Monitor::from_point(x, y).map_err(e)?;
            let mx = m.x().map_err(e)?;
            let my = m.y().map_err(e)?;
            m.capture_region((x - mx).max(0) as u32, (y - my).max(0) as u32, w, h).map_err(e)?
        }
    };
    ensure_parent(path)?;
    img.save(path).map_err(|x| format!("no se pudo guardar '{}': {}", path, x))
}

pub fn pixel(x: i32, y: i32) -> Result<String, String> {
    let m = xcap::Monitor::from_point(x, y).map_err(e)?;
    let mx = m.x().map_err(e)?;
    let my = m.y().map_err(e)?;
    let img = m.capture_region((x - mx) as u32, (y - my) as u32, 1, 1).map_err(e)?;
    let p = img.get_pixel(0, 0);
    Ok(format!("#{:02X}{:02X}{:02X}", p[0], p[1], p[2]))
}

pub fn ensure_parent(path: &str) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(e)?;
        }
    }
    Ok(())
}

// ---------- Ventanas ----------

pub struct WinInfo {
    pub id: u32,
    pub title: String,
    pub app: String,
    pub focused: bool,
}

pub fn windows() -> Result<Vec<WinInfo>, String> {
    let mut out = Vec::new();
    for w in xcap::Window::all().map_err(e)? {
        let title = w.title().unwrap_or_default();
        let app = w.app_name().unwrap_or_default();
        if title.trim().is_empty() && app.trim().is_empty() {
            continue;
        }
        out.push(WinInfo {
            id: w.id().unwrap_or(0),
            title,
            app,
            focused: w.is_focused().unwrap_or(false),
        });
    }
    Ok(out)
}

pub fn find_window(query: &str) -> Result<Option<WinInfo>, String> {
    let q = query.to_lowercase();
    Ok(windows()?
        .into_iter()
        .find(|w| w.title.to_lowercase().contains(&q) || w.app.to_lowercase().contains(&q)))
}

pub fn focus_window(query: &str) -> Result<(), String> {
    let w = find_window(query)?.ok_or_else(|| format!("no hay ninguna ventana que coincida con '{}'", query))?;
    focus_impl(&w)
}

#[cfg(target_os = "windows")]
fn focus_impl(w: &WinInfo) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_MENU};
    use windows_sys::Win32::UI::WindowsAndMessaging::{IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE};
    let hwnd = w.id as usize as windows_sys::Win32::Foundation::HWND;
    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        // Truco habitual para que Windows permita cambiar la ventana en primer plano.
        keybd_event(VK_MENU as u8, 0, 0, 0);
        keybd_event(VK_MENU as u8, 0, KEYEVENTF_KEYUP, 0);
        if SetForegroundWindow(hwnd) == 0 {
            return Err(format!("no se pudo activar la ventana '{}'", w.title));
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn focus_impl(w: &WinInfo) -> Result<(), String> {
    let script = format!("tell application \"{}\" to activate", w.app.replace('"', "\\\""));
    run_status(Command::new("osascript").arg("-e").arg(script))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn focus_impl(w: &WinInfo) -> Result<(), String> {
    let id = format!("0x{:x}", w.id);
    run_status(Command::new("wmctrl").args(["-i", "-a", &id]))
        .or_else(|_| run_status(Command::new("xdotool").args(["windowactivate", &w.id.to_string()])))
        .map_err(|_| "para FOCUS en Linux instala wmctrl o xdotool".to_string())
}

#[allow(dead_code)]
fn run_status(cmd: &mut Command) -> Result<(), String> {
    let st = cmd.status().map_err(e)?;
    if st.success() { Ok(()) } else { Err(format!("el comando terminó con código {:?}", st.code())) }
}

// ---------- Procesos ----------

pub fn open(target: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let r = {
        use std::os::windows::process::CommandExt;
        Command::new("cmd").args(["/C", "start", ""]).arg(target).creation_flags(0x0800_0000).spawn()
    };
    #[cfg(target_os = "macos")]
    let r = Command::new("open").arg(target).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = Command::new("xdg-open").arg(target).spawn();
    r.map(|_| ()).map_err(|x| format!("no se pudo abrir '{}': {}", target, x))
}

pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

pub fn exec(cmdline: &str) -> Result<ExecResult, String> {
    #[cfg(target_os = "windows")]
    let out = {
        use std::os::windows::process::CommandExt;
        Command::new("cmd").arg("/C").raw_arg(cmdline).output()
    };
    #[cfg(not(target_os = "windows"))]
    let out = Command::new("sh").arg("-c").arg(cmdline).output();
    let out = out.map_err(|x| format!("no se pudo ejecutar '{}': {}", cmdline, x))?;
    Ok(ExecResult {
        stdout: decode(&out.stdout),
        stderr: decode(&out.stderr),
        code: out.status.code().unwrap_or(-1),
    })
}

fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => decode_native(bytes),
    }
}

/// La consola de Windows escribe en la página de códigos OEM (p. ej. 850), no en UTF-8.
#[cfg(target_os = "windows")]
fn decode_native(bytes: &[u8]) -> String {
    use windows_sys::Win32::Globalization::{MultiByteToWideChar, CP_OEMCP};
    unsafe {
        let n = MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), bytes.len() as i32, std::ptr::null_mut(), 0);
        if n <= 0 {
            return String::from_utf8_lossy(bytes).to_string();
        }
        let mut buf = vec![0u16; n as usize];
        MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), bytes.len() as i32, buf.as_mut_ptr(), n);
        String::from_utf16_lossy(&buf)
    }
}

#[cfg(not(target_os = "windows"))]
fn decode_native(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Usa píxeles físicos para que ratón y capturas coincidan con pantallas escaladas (125 %, 150 %...).
#[cfg(target_os = "windows")]
pub fn init_platform() {
    use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn init_platform() {}

// ---------- Diálogos y notificaciones ----------

pub fn alert(msg: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST};
        let text: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
        let title: Vec<u16> = "acm".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION | MB_TOPMOST | MB_SETFOREGROUND);
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display dialog \"{}\" with title \"acm\" buttons {{\"OK\"}} default button 1",
            msg.replace('\\', "\\\\").replace('"', "\\\"")
        );
        run_status(Command::new("osascript").arg("-e").arg(script))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        run_status(Command::new("zenity").args(["--info", "--title=acm", "--text"]).arg(msg))
            .or_else(|_| run_status(Command::new("kdialog").args(["--title", "acm", "--msgbox"]).arg(msg)))
            .map_err(|_| "para ALERT en Linux instala zenity o kdialog".to_string())
    }
}

pub fn notify(msg: &str, title: &str) -> Result<(), String> {
    notify_rust::Notification::new()
        .summary(title)
        .body(msg)
        .show()
        .map(|_| ())
        .map_err(|x| format!("no se pudo mostrar la notificación: {}", x))
}
