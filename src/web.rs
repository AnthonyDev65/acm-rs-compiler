//! Automatización web con Chrome/Edge a través del protocolo DevTools (sin WebDriver).

use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use headless_chrome::browser::tab::element::Element;
use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption;
use headless_chrome::{Browser, LaunchOptions, Tab};

use crate::value::Value;

fn e<E: std::fmt::Display>(x: E) -> String {
    x.to_string()
}

/// Busca el elemento con un selector CSS o XPath (los que empiezan por `/` o `(`).
const FIND_JS: &str = r#"function __acmFind(s){
  if (s.startsWith('/') || s.startsWith('(') || s.startsWith('./')) {
    return document.evaluate(s, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue;
  }
  return document.querySelector(s);
}"#;

pub struct Web {
    exe: Option<PathBuf>,
    browser_name: String,
    headless: bool,
    profile: Option<PathBuf>,
    pub timeout: Duration,
    browser: Option<Browser>,
    tabs: Vec<Arc<Tab>>,
    cur: usize,
}

pub fn is_xpath(sel: &str) -> bool {
    sel.starts_with('/') || sel.starts_with('(') || sel.starts_with("./")
}

fn normalize_url(url: &str) -> String {
    let u = url.trim();
    if u.contains("://") || u.starts_with("about:") || u.starts_with("data:") || u.starts_with("file:") || u.starts_with("chrome:") {
        u.to_string()
    } else {
        format!("http://{}", u)
    }
}

impl Web {
    pub fn new() -> Self {
        Web {
            exe: None,
            browser_name: "CHROME".into(),
            headless: false,
            profile: None,
            timeout: Duration::from_secs(10),
            browser: None,
            tabs: Vec::new(),
            cur: 0,
        }
    }

    pub fn is_running(&self) -> bool {
        self.browser.is_some()
    }

    /// `WEBBROWSER CHROME|EDGE|<ruta> [HEADLESS|VISIBLE]`
    pub fn configure(&mut self, spec: &str) -> Result<(), String> {
        let mut words: Vec<String> = Vec::new();
        let mut path_part = String::new();
        let t = spec.trim();
        let rest = if let Some(stripped) = t.strip_prefix('"') {
            let end = stripped.find('"').ok_or("falta una comilla en WEBBROWSER")?;
            path_part = stripped[..end].to_string();
            &stripped[end + 1..]
        } else {
            t
        };
        words.extend(rest.split_whitespace().map(|s| s.to_ascii_uppercase()));
        let mut kind = if path_part.is_empty() { None } else { Some(path_part) };
        let mut headless = false;
        for w in words {
            match w.as_str() {
                "HEADLESS" | "OCULTO" => headless = true,
                "VISIBLE" => headless = false,
                "CHROME" | "EDGE" | "FIREFOX" | "CHROMIUM" | "BRAVE" => kind = Some(w),
                other => kind = Some(other.to_string()),
            }
        }
        let kind = kind.unwrap_or_else(|| "CHROME".into());
        let exe = match kind.as_str() {
            "CHROME" | "CHROMIUM" => None,
            "EDGE" => Some(find_edge().ok_or("no se encontró Microsoft Edge instalado")?),
            "BRAVE" => Some(find_brave().ok_or("no se encontró Brave instalado")?),
            "FIREFOX" => return Err("Firefox no está soportado; usa WEBBROWSER CHROME o EDGE".into()),
            path => Some(PathBuf::from(path)),
        };
        let changed = exe != self.exe || headless != self.headless;
        self.exe = exe;
        self.headless = headless;
        self.browser_name = kind;
        if changed && self.is_running() {
            self.close();
        }
        Ok(())
    }

    pub fn set_profile(&mut self, dir: &str) {
        self.profile = if dir.trim().is_empty() { None } else { Some(PathBuf::from(dir)) };
        if self.is_running() {
            self.close();
        }
    }

    fn ensure(&mut self) -> Result<(), String> {
        if let Some(b) = &self.browser {
            if b.get_version().is_ok() && !self.tabs.is_empty() {
                return Ok(());
            }
            self.close();
        }
        let path = match &self.exe {
            Some(p) => p.clone(),
            None => headless_chrome::browser::default_executable()
                .map_err(|x| format!("no se encontró Chrome ({}). Instálalo o usa WEBBROWSER EDGE", x))?,
        };
        let args: Vec<&OsStr> = vec![
            OsStr::new("--start-maximized"),
            OsStr::new("--no-first-run"),
            OsStr::new("--no-default-browser-check"),
            OsStr::new("--disable-blink-features=AutomationControlled"),
        ];
        let opts = LaunchOptions::default_builder()
            .headless(self.headless)
            .path(Some(path))
            .args(args)
            .window_size(if self.headless { Some((1366, 900)) } else { None })
            .user_data_dir(self.profile.clone())
            .idle_browser_timeout(Duration::from_secs(60 * 60 * 24 * 30))
            .build()
            .map_err(e)?;
        let browser = Browser::new(opts).map_err(|x| format!("no se pudo iniciar {}: {}", self.browser_name, x))?;
        let tab = browser.new_tab().map_err(e)?;
        // Cerrar pestañas sobrantes que haya abierto el navegador al iniciar.
        if let Ok(all) = browser.get_tabs().lock() {
            for t in all.iter() {
                if t.get_target_id() != tab.get_target_id() {
                    let _ = t.close(false);
                }
            }
        }
        tab.set_default_timeout(self.timeout);
        let _ = tab.activate();
        self.tabs = vec![tab];
        self.cur = 0;
        self.browser = Some(browser);
        Ok(())
    }

    fn tab(&mut self) -> Result<Arc<Tab>, String> {
        self.ensure()?;
        Ok(self.tabs[self.cur].clone())
    }

    pub fn close(&mut self) {
        for t in self.tabs.drain(..) {
            let _ = t.close(false);
        }
        self.browser = None;
        self.cur = 0;
    }

    pub fn open(&mut self, url: &str, nowait: bool) -> Result<(), String> {
        let tab = self.tab()?;
        let url = normalize_url(url);
        if nowait {
            let js = format!("window.location.href = {}", serde_json::to_string(&url).unwrap());
            tab.evaluate(&js, false).map_err(e)?;
        } else {
            tab.navigate_to(&url).map_err(e)?;
            tab.wait_until_navigated().map_err(|x| format!("la página no terminó de cargar: {}", x))?;
        }
        Ok(())
    }

    pub fn new_tab(&mut self, url: Option<&str>, nowait: bool) -> Result<(), String> {
        self.ensure()?;
        let tab = self.browser.as_ref().unwrap().new_tab().map_err(e)?;
        tab.set_default_timeout(self.timeout);
        self.tabs.push(tab.clone());
        self.cur = self.tabs.len() - 1;
        let _ = tab.activate();
        if let Some(u) = url {
            self.open(u, nowait)?;
        }
        Ok(())
    }

    pub fn close_tab(&mut self) -> Result<(), String> {
        self.ensure()?;
        if self.tabs.len() <= 1 {
            return Err("no se puede cerrar la última pestaña (usa WEBCLOSE)".into());
        }
        let t = self.tabs.remove(self.cur);
        let _ = t.close(false);
        self.cur = 0;
        let _ = self.tabs[0].activate();
        Ok(())
    }

    pub fn switch_tab(&mut self, i: usize) -> Result<(), String> {
        self.ensure()?;
        if i >= self.tabs.len() {
            return Err(format!("la pestaña {} no existe (hay {}, se cuentan desde 0)", i, self.tabs.len()));
        }
        self.cur = i;
        let _ = self.tabs[i].activate();
        Ok(())
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    fn find<'a>(&self, tab: &'a Tab, sel: &str, timeout: Duration) -> Result<Element<'a>, String> {
        // Sondeo propio por JavaScript (más rápido que las esperas de headless_chrome).
        self.wait_state(tab, sel, "EXISTS", timeout)
            .map_err(|_| format!("no se encontró el elemento '{}' (esperé {} s)", sel, timeout.as_secs_f32()))?;
        let r = if is_xpath(sel) { tab.find_element_by_xpath(sel) } else { tab.find_element(sel) };
        r.map_err(|x| format!("no se pudo acceder al elemento '{}': {}", sel, x))
    }

    pub fn click(&mut self, sel: &str) -> Result<(), String> {
        let tab = self.tab()?;
        self.wait_state(&tab, sel, "CLICKABLE", self.timeout)?;
        let el = self.find(&tab, sel, self.timeout)?;
        let _ = el.scroll_into_view();
        if el.click().is_err() {
            el.call_js_fn("function(){ this.click(); }", vec![], false).map_err(e)?;
        }
        Ok(())
    }

    pub fn input(&mut self, sel: &str, text: &str) -> Result<(), String> {
        let tab = self.tab()?;
        let el = self.find(&tab, sel, self.timeout)?;
        let _ = el.scroll_into_view();
        el.call_js_fn(
            "function(){ this.focus(); if ('value' in this) { this.value = ''; } this.dispatchEvent(new Event('input', {bubbles:true})); }",
            vec![],
            false,
        )
        .map_err(e)?;
        // insertText: una sola llamada que dispara los eventos `input` (como Puppeteer sendCharacter).
        tab.send_character(text).map_err(e)?;
        let _ = el.call_js_fn("function(){ this.dispatchEvent(new Event('change', {bubbles:true})); }", vec![], false);
        Ok(())
    }

    pub fn get_text(&mut self, sel: &str) -> Result<String, String> {
        let tab = self.tab()?;
        let el = self.find(&tab, sel, self.timeout)?;
        let r = el
            .call_js_fn(
                "function(){ const t = this.tagName; if (t === 'INPUT' || t === 'TEXTAREA' || t === 'SELECT') return this.value; return this.innerText; }",
                vec![],
                false,
            )
            .map_err(e)?;
        Ok(r.value.and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
    }

    pub fn attr(&mut self, sel: &str, name: &str) -> Result<Value, String> {
        let tab = self.tab()?;
        let el = self.find(&tab, sel, self.timeout)?;
        Ok(el.get_attribute_value(name).map_err(e)?.map(Value::Str).unwrap_or(Value::Null))
    }

    fn state(&self, tab: &Tab, sel: &str) -> Result<String, String> {
        let js = format!(
            "(() => {{ {} const el = __acmFind({}); if (!el) return 'none';
               const st = getComputedStyle(el);
               const vis = !!(el.offsetWidth || el.offsetHeight || el.getClientRects().length) && st.visibility !== 'hidden' && st.display !== 'none';
               if (!vis) return 'hidden';
               return el.disabled ? 'visible' : 'clickable'; }})()",
            FIND_JS,
            serde_json::to_string(sel).unwrap()
        );
        let r = tab.evaluate(&js, false).map_err(e)?;
        Ok(r.value.and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "none".into()))
    }

    fn wait_state(&self, tab: &Tab, sel: &str, cond: &str, timeout: Duration) -> Result<(), String> {
        let start = Instant::now();
        loop {
            let st = self.state(tab, sel).unwrap_or_else(|_| "none".into());
            let ok = match cond {
                "EXISTS" => st != "none",
                "VISIBLE" => st == "visible" || st == "clickable",
                "HIDDEN" => st == "none" || st == "hidden",
                "GONE" => st == "none",
                "CLICKABLE" => st == "clickable",
                other => return Err(format!("condición WEBWAIT no soportada: {}", other)),
            };
            if ok {
                return Ok(());
            }
            if start.elapsed() >= timeout {
                return Err(format!("tiempo agotado esperando que '{}' esté {} (estado: {})", sel, cond, st));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn wait(&mut self, sel: &str, cond: &str, timeout: Option<Duration>) -> Result<(), String> {
        let tab = self.tab()?;
        self.wait_state(&tab, sel, cond, timeout.unwrap_or(self.timeout))
    }

    pub fn exists(&mut self, sel: &str) -> Result<bool, String> {
        let tab = self.tab()?;
        Ok(self.state(&tab, sel)? != "none")
    }

    pub fn select(&mut self, sel: &str, option: &str) -> Result<(), String> {
        let tab = self.tab()?;
        let el = self.find(&tab, sel, self.timeout)?;
        let r = el
            .call_js_fn(
                "function(t){ for (const o of this.options) { if (o.text.trim() === t || o.value === t) { this.value = o.value; this.dispatchEvent(new Event('input',{bubbles:true})); this.dispatchEvent(new Event('change',{bubbles:true})); return true; } } return false; }",
                vec![serde_json::json!(option)],
                false,
            )
            .map_err(e)?;
        if r.value.and_then(|v| v.as_bool()) == Some(true) {
            Ok(())
        } else {
            Err(format!("la lista '{}' no tiene la opción '{}'", sel, option))
        }
    }

    pub fn check(&mut self, sel: &str, want: bool) -> Result<(), String> {
        let tab = self.tab()?;
        let el = self.find(&tab, sel, self.timeout)?;
        let r = el.call_js_fn("function(){ return !!this.checked; }", vec![], false).map_err(e)?;
        let checked = r.value.and_then(|v| v.as_bool()).unwrap_or(false);
        if checked != want {
            let _ = el.scroll_into_view();
            if el.click().is_err() {
                el.call_js_fn("function(){ this.click(); }", vec![], false).map_err(e)?;
            }
        }
        Ok(())
    }

    pub fn scroll_to(&mut self, sel: &str) -> Result<(), String> {
        let tab = self.tab()?;
        let el = self.find(&tab, sel, self.timeout)?;
        el.scroll_into_view().map_err(e)?;
        Ok(())
    }

    pub fn screenshot(&mut self, path: &str) -> Result<(), String> {
        let tab = self.tab()?;
        let data = tab.capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true).map_err(e)?;
        crate::host::ensure_parent(path)?;
        std::fs::write(path, data).map_err(|x| format!("no se pudo guardar '{}': {}", path, x))
    }

    pub fn js(&mut self, code: &str) -> Result<Value, String> {
        let tab = self.tab()?;
        let src = if code.contains("return ") || code.contains("return;") {
            format!("(async () => {{ {} }})()", code)
        } else {
            code.to_string()
        };
        let r = tab.evaluate(&src, true).map_err(|x| format!("error de JavaScript: {}", x))?;
        Ok(r.value.as_ref().map(Value::from_json).unwrap_or(Value::Null))
    }

    pub fn key(&mut self, key: &str) -> Result<(), String> {
        let tab = self.tab()?;
        tab.press_key(key).map_err(e)?;
        Ok(())
    }

    pub fn url(&mut self) -> Result<String, String> {
        Ok(self.tab()?.get_url())
    }

    pub fn title(&mut self) -> Result<String, String> {
        self.tab()?.get_title().map_err(e)
    }

    pub fn back(&mut self) -> Result<(), String> {
        self.js("history.back()")?;
        Ok(())
    }

    pub fn reload(&mut self) -> Result<(), String> {
        let tab = self.tab()?;
        tab.reload(false, None).map_err(e)?;
        Ok(())
    }
}

fn find_edge() -> Option<PathBuf> {
    let candidates: &[&str] = if cfg!(target_os = "windows") {
        &[
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ]
    } else if cfg!(target_os = "macos") {
        &["/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"]
    } else {
        &["/usr/bin/microsoft-edge", "/usr/bin/microsoft-edge-stable", "/opt/microsoft/msedge/msedge"]
    };
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

fn find_brave() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if cfg!(target_os = "windows") {
        for base in [std::env::var("ProgramFiles").ok(), std::env::var("LOCALAPPDATA").ok()].into_iter().flatten() {
            candidates.push(PathBuf::from(base).join(r"BraveSoftware\Brave-Browser\Application\brave.exe"));
        }
    } else if cfg!(target_os = "macos") {
        candidates.push("/Applications/Brave Browser.app/Contents/MacOS/Brave Browser".into());
    } else {
        candidates.push("/usr/bin/brave-browser".into());
    }
    candidates.into_iter().find(|p| p.exists())
}
