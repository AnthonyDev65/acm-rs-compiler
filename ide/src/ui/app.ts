// Controlador de la aplicación: une el editor, el explorador y la salida con el intérprete.

import { engine, type AcmEvent, type EngineInfo, type RunMessage, type RunRequest } from "../core/engine";
import { basename, dirname, files, join, samePath, type Entry } from "../core/files";
import { launch } from "../core/launch";
import { saveSettings, settings } from "../core/settings";
import { appWindow } from "../core/window";
import { acm, type CheckResult } from "../lang/acm";
import { EXTENSION_TEMPLATE, SCRIPT_TEMPLATE, headerInsertLine, isExtension, setDirective } from "../lang/templates";
import { argsDialog, confirmDialog, metaDialog, newArgDialog, promptText, settingsDialog, unsavedDialog } from "./dialogs";
import { h, icon, mark } from "./dom";
import { EditorArea, type OpenDoc } from "./editor";
import { Explorer } from "./explorer";
import { FormPanel } from "./form-panel";
import { OutputPanel } from "./output";
import { Titlebar } from "./titlebar";

interface ActiveRun {
  id: number | null;
  doc: OpenDoc | null;
  eval: boolean;
  finished: boolean;
}

export class App {
  private editor: EditorArea;
  private explorer: Explorer;
  private output = new OutputPanel();
  private engineInfo: EngineInfo | null = null;
  private run: ActiveRun | null = null;
  /** Mensajes que llegan antes de conocer el id de la ejecución. */
  private early: RunMessage[] = [];
  private lastCheck: { source: string; result: CheckResult | null } | null = null;

  private titlebar: Titlebar;
  private form: FormPanel;
  private statusRun = h("span", { class: "status-run" });
  private statusPos = h("span", {});
  private statusEngine = h("button", { class: "status-engine", title: "Ajustes del intérprete" });

  constructor(root: HTMLElement) {
    const card = (ico: "script" | "extension" | "folderOpen", title: string, desc: string, fn: () => void) =>
      h("button", { type: "button", class: "welcome-card", onclick: fn }, h("span", { class: "card-icon" }, icon(ico, 20)), h("strong", {}, title), h("span", {}, desc));
    const welcome = h("div", { class: "welcome" },
      h("div", { class: "welcome-inner" },
        h("div", { class: "welcome-mark" }, mark(28)),
        h("h1", {}, "Automatiza tu escritorio"),
        h("p", {}, "Ratón, teclado, ventanas y navegador con scripts sencillos."),
        h("div", { class: "welcome-cards" },
          card("script", "Nuevo script", "Un archivo .acm para empezar a automatizar", () => this.newFile()),
          card("extension", "Nueva extensión", "Un .acmx con su propia ventana de opciones", () => this.newExtension()),
          card("folderOpen", "Abrir carpeta", "Trabaja con tus scripts existentes", () => void this.openFolder()),
        ),
        h("dl", { class: "shortcuts" },
          ...[["F5", "Ejecutar"], ["Ctrl+F5", "Simular sin mover nada"], ["Shift+F5", "Detener"], ["Ctrl+Espacio", "Autocompletar"], ["Ctrl+`", "Consola"]]
            .flatMap(([k, v]) => [h("dt", {}, h("kbd", {}, k)), h("dd", {}, v)]),
        ),
      ),
    );
    this.titlebar = new Titlebar({
      newScript: () => this.newFile(),
      newExtension: () => this.newExtension(),
      openFile: () => void this.openFileDialog(),
      openFolder: () => void this.openFolder(),
      save: () => void this.save(),
      saveAs: () => void this.save(true),
      run: () => void this.runActive(false),
      dry: () => void this.runActive(true),
      stop: () => void this.stop(),
      settings: () => void this.openSettings(),
      toggleSidebar: () => this.toggleSidebar(),
      toggleForm: () => this.toggleForm(),
      options: () => [
        { label: "Mostrar cada paso en la salida", checked: settings.showSteps, toggle: () => saveSettings({ showSteps: !settings.showSteps }) },
        { label: "Seguir tras errores (--keep-going)", checked: settings.keepGoing, toggle: () => saveSettings({ keepGoing: !settings.keepGoing }) },
        { label: "ALERT y NOTIFY en la salida (--no-dialogs)", checked: settings.noDialogs, toggle: () => saveSettings({ noDialogs: !settings.noDialogs }) },
        { label: "Sin parada de emergencia (--no-failsafe)", checked: settings.noFailsafe, toggle: () => saveSettings({ noFailsafe: !settings.noFailsafe }) },
      ],
    });
    this.form = new FormPanel({
      info: (src) => engine.info(src),
      saved: () => (this.editor.active ? settings.argValues[this.argKey(this.editor.active)] ?? {} : {}),
      save: (values) => this.editor.active && saveSettings({ argValues: { ...settings.argValues, [this.argKey(this.editor.active)]: values } }),
      run: (values) => void this.runActive(false, values),
      addArg: (kind, taken) => void this.addArg(kind, taken),
      editMeta: (info) => void this.editMeta(info),
    });
    this.editor = new EditorArea(acm((src) => this.check(src)), welcome);
    this.editor.onChange = () => this.refreshChrome();
    this.editor.onCursor = (l, c) => (this.statusPos.textContent = `Ln ${l}, Col ${c}`);
    this.editor.onCloseRequest = (d) => void this.closeDoc(d);

    this.explorer = new Explorer({
      open: (p) => void this.openPath(p),
      openFolder: () => void this.openFolder(),
      newEntry: (dir, isDir) => void this.newEntry(dir, isDir),
      rename: (e) => void this.renameEntry(e),
      remove: (e) => void this.removeEntry(e),
      error: (m) => this.output.write("error", m),
    });

    this.output.onSubmit = (t) => void this.consoleInput(t);
    this.output.onGoto = (l) => this.editor.gotoLine(l);
    this.statusEngine.addEventListener("click", () => void this.openSettings());

    root.append(
      this.titlebar.el,
      h("main", { class: "workbench" },
        this.explorer.el,
        this.splitter("x"),
        h("div", { class: "center" }, this.editor.el, this.splitter("y"), this.output.el),
        this.form.el,
      ),
      h("footer", { class: "statusbar" }, this.statusRun, h("div", { class: "spacer" }), this.statusPos, this.statusEngine),
    );
    document.documentElement.style.setProperty("--editor-font", `${settings.fontSize}px`);
    document.body.classList.toggle("no-sidebar", settings.sidebarHidden);
    window.addEventListener("keydown", (e) => this.onKey(e), true);
    appWindow.onCloseRequested(() => this.confirmCloseAll());
  }

  async start() {
    void engine.onMessage((m) => this.onRunMessage(m));
    await this.locateEngine();
    await this.explorer.setRoot(settings.workspace);
    for (const p of settings.openFiles) await this.openPath(p, true);
    const active = settings.activeFile && this.editor.find(settings.activeFile, samePath);
    if (active) this.editor.activate(active);
    // Archivo pedido con `--edit` o desde la ventana de una extensión.
    const requested = await launch.takeOpenRequest().catch(() => null);
    if (requested) await this.openPath(requested);
    void launch.onOpenRequest((p) => void this.openPath(p));
    this.refreshChrome();
  }

  // ---------- Interfaz ----------

  private toggleSidebar() {
    saveSettings({ sidebarHidden: !settings.sidebarHidden });
    document.body.classList.toggle("no-sidebar", settings.sidebarHidden);
  }

  private toggleForm() {
    saveSettings({ formHidden: !settings.formHidden });
    this.refreshChrome();
  }

  /** Clave con la que se guardan los últimos valores de los @ARG de un documento. */
  private argKey(doc: OpenDoc): string {
    return doc.path ?? doc.name;
  }

  /** Separador arrastrable: "x" ajusta el ancho del explorador, "y" el alto de la salida. */
  private splitter(axis: "x" | "y"): HTMLElement {
    const el = h("div", { class: `splitter ${axis}`, role: "separator", "aria-orientation": axis === "x" ? "vertical" : "horizontal" });
    const prop = axis === "x" ? "--sidebar-w" : "--output-h";
    const saved = localStorage.getItem(`acm-ide${prop}`);
    if (saved) document.documentElement.style.setProperty(prop, saved);
    el.addEventListener("pointerdown", (e) => {
      el.setPointerCapture(e.pointerId);
      document.body.classList.add(`resizing-${axis}`);
      const move = (ev: PointerEvent) => {
        const v = axis === "x" ? Math.min(Math.max(ev.clientX - 6, 180), 520) : Math.min(Math.max(window.innerHeight - ev.clientY - 30, 110), window.innerHeight - 220);
        document.documentElement.style.setProperty(prop, `${v}px`);
      };
      const up = () => {
        el.removeEventListener("pointermove", move);
        document.body.classList.remove(`resizing-${axis}`);
        localStorage.setItem(`acm-ide${prop}`, document.documentElement.style.getPropertyValue(prop));
      };
      el.addEventListener("pointermove", move);
      el.addEventListener("pointerup", up, { once: true });
    });
    return el;
  }

  private formDoc: OpenDoc | null = null;

  private refreshChrome() {
    this.editor.render();
    const doc = this.editor.active;
    this.explorer.setActive(doc?.path ?? null);
    const running = this.run !== null && !this.run.eval;
    const dirty = !!doc && this.editor.isDirty(doc);
    const source = doc ? this.editor.text(doc) : "";
    const ext = !!doc && isExtension(doc.name, source);
    this.titlebar.setDoc(doc?.name ?? null, dirty, ext);
    this.titlebar.setRunState(!!doc && this.run === null && !!this.engineInfo, running);

    // Panel con la ventana de la extensión.
    const showForm = ext && !settings.formHidden && !!this.engineInfo;
    document.body.classList.toggle("with-form", showForm);
    this.titlebar.setFormActive(showForm);
    if (showForm && doc) {
      if (this.formDoc !== doc) {
        this.form.reset();
        this.formDoc = doc;
        this.form.update(source, doc.name.replace(/\.acmx?$/i, ""), true);
      } else {
        this.form.update(source, doc.name.replace(/\.acmx?$/i, ""));
      }
    } else {
      this.formDoc = null;
    }

    void appWindow.setTitle(doc ? `${dirty ? "● " : ""}${doc.name} — acm IDE` : "acm IDE");
    saveSettings({
      openFiles: this.editor.docs.flatMap((d) => (d.path ? [d.path] : [])),
      activeFile: doc?.path ?? null,
    });
  }

  private onKey(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    const k = e.key.toLowerCase();
    const run = (fn: () => unknown) => {
      e.preventDefault();
      e.stopPropagation();
      void fn();
    };
    if (e.key === "F5" && e.shiftKey) run(() => this.stop());
    else if (e.key === "F5" && mod) run(() => this.runActive(true));
    else if (e.key === "F5") run(() => this.runActive(false));
    else if (mod && k === "s" && e.shiftKey) run(() => this.save(true));
    else if (mod && k === "s") run(() => this.save());
    else if (mod && k === "n" && e.shiftKey) run(() => this.newExtension());
    else if (mod && k === "n") run(() => this.newFile());
    else if (mod && k === "b") run(() => this.toggleSidebar());
    else if (mod && k === "o") run(() => this.openFileDialog());
    else if (mod && k === "w") run(() => this.editor.active && this.closeDoc(this.editor.active));
    else if (mod && (e.key === "`" || e.code === "Backquote")) run(() => this.output.focusInput());
  }

  // ---------- Intérprete ----------

  private async locateEngine() {
    try {
      this.engineInfo = await engine.locate(settings.acmPath || undefined);
      this.statusEngine.textContent = this.engineInfo.version;
      this.statusEngine.classList.remove("bad");
      this.statusEngine.title = this.engineInfo.path;
    } catch (e) {
      this.engineInfo = null;
      this.statusEngine.textContent = "acm no encontrado";
      this.statusEngine.classList.add("bad");
      this.output.write("error", String(e));
    }
    this.lastCheck = null;
    this.refreshChrome();
  }

  private async openSettings() {
    const p = await settingsDialog(settings.acmPath, (path) => engine.locate(path || undefined), {
      check: () => launch.acmxAssociated(),
      associate: () => launch.associateAcmx(),
    });
    if (p !== null) saveSettings({ acmPath: p });
    await this.locateEngine();
  }

  private async check(source: string): Promise<CheckResult | null> {
    if (!this.engineInfo) return null;
    if (this.lastCheck?.source === source) return this.lastCheck.result;
    let result: CheckResult | null = null;
    try {
      result = await engine.check(source);
    } catch {
      result = null;
    }
    this.lastCheck = { source, result };
    return result;
  }

  /** Ejecuta el documento activo. `values` son los @ARG ya elegidos en el panel; si faltan, se abre la ventana. */
  private async runActive(dryRun: boolean, values?: Record<string, string>) {
    const doc = this.editor.active;
    if (!doc || this.run || !this.engineInfo) return;
    if (doc.path && this.editor.isDirty(doc) && !(await this.save())) return;
    const source = this.editor.text(doc);

    let args: string[] = [];
    if (/^\s*@ARG/im.test(source)) {
      try {
        if (!values) {
          const info = await engine.info(source);
          if (info.args.length) {
            const chosen = await argsDialog(info, doc.name.replace(/\.acmx?$/i, ""), settings.argValues[this.argKey(doc)] ?? {});
            if (!chosen) return;
            values = chosen;
          }
        }
        if (values) {
          saveSettings({ argValues: { ...settings.argValues, [this.argKey(doc)]: values } });
          args = Object.entries(values).map(([k, v]) => `${k}=${v}`);
        }
      } catch (e) {
        this.output.write("error", String(e));
        return;
      }
    }

    this.editor.setErrorLine(doc, null);
    this.output.separator();
    this.output.write("info", `${dryRun ? "Simulando" : "Ejecutando"} ${doc.name}${args.length ? "  " + args.join(" ") : ""}`);
    await this.startRun(
      {
        ...(doc.path ? { path: doc.path } : { source }),
        cwd: doc.path ? dirname(doc.path) : settings.workspace ?? undefined,
        args,
        dryRun,
        trace: true,
        keepGoing: settings.keepGoing,
        noDialogs: settings.noDialogs,
        noFailsafe: settings.noFailsafe,
      },
      doc,
      false,
    );
  }

  private async startRun(req: RunRequest, doc: OpenDoc | null, isEval: boolean) {
    this.run = { id: null, doc, eval: isEval, finished: false };
    this.early = [];
    this.setRunStatus(isEval ? "Evaluando…" : "En ejecución…", "busy");
    this.refreshChrome();
    try {
      const id = await engine.run(req);
      this.run.id = id;
      const early = this.early.filter((m) => m.runId === id);
      this.early = [];
      early.forEach((m) => this.onRunMessage(m));
    } catch (e) {
      this.output.write("error", String(e));
      this.run = null;
      this.setRunStatus("", "");
      this.refreshChrome();
    }
  }

  private async stop() {
    if (this.run?.id != null && !this.run.eval) {
      await engine.stop(this.run.id).catch((e) => this.output.write("error", String(e)));
    }
  }

  private async consoleInput(text: string) {
    const r = this.run;
    if (r?.id != null && r.finished === false && this.asking) {
      this.output.write("input", text);
      this.asking = false;
      this.output.setPrompt(null);
      await engine.input(r.id, text).catch((e) => this.output.write("error", String(e)));
      return;
    }
    if (r) {
      this.output.write("warn", "Hay una ejecución en curso; espera a que termine o detenla.");
      return;
    }
    if (!this.engineInfo) {
      this.output.write("error", "No se encontró el intérprete acm. Revísalo en Ajustes.");
      return;
    }
    this.output.write("input", `acm> ${text}`);
    await this.startRun({ eval: text, cwd: settings.workspace ?? undefined, noDialogs: settings.noDialogs, noFailsafe: settings.noFailsafe }, null, true);
  }

  private asking = false;

  private onRunMessage(m: RunMessage) {
    const r = this.run;
    if (!r) return;
    if (r.id === null) {
      this.early.push(m);
      return;
    }
    if (m.runId !== r.id) return;
    switch (m.kind) {
      case "event":
        this.onAcmEvent(r, m.event);
        break;
      case "stdout":
        this.output.write("print", m.text);
        break;
      case "stderr":
        this.output.write("stderr", m.text);
        break;
      case "exit":
        if (!r.finished && !r.eval) this.output.write("fail", "■ Detenido");
        this.editor.setRunLine(r.doc, null);
        this.asking = false;
        this.output.setPrompt(null);
        this.run = null;
        this.setRunStatus(r.finished ? this.statusRun.textContent ?? "" : "Detenido", r.finished ? this.statusRun.dataset.state ?? "" : "bad");
        this.refreshChrome();
        break;
    }
  }

  private onAcmEvent(r: ActiveRun, event: AcmEvent) {
    const ev = event as Record<string, any>;
    const line = typeof ev.line === "number" ? ev.line : null;
    switch (ev.event) {
      case "print":
        this.output.write("print", String(ev.text));
        break;
      case "step":
        this.editor.setRunLine(r.doc, line);
        if (settings.showSteps && !r.eval) this.output.write("step", String(ev.code), line);
        break;
      case "dry":
        this.output.write("dry", String(ev.action), line);
        break;
      case "warning":
        this.output.write("warn", String(ev.message));
        break;
      case "error":
        this.output.write("error", String(ev.message), r.eval ? null : line);
        if (!r.eval) this.editor.setErrorLine(r.doc, line);
        break;
      case "ask":
        this.asking = true;
        this.output.setPrompt(String(ev.prompt).trim() || `${ev.var}:`);
        break;
      case "screenshot":
        this.output.write("info", `Captura guardada: ${ev.path}`);
        break;
      case "done": {
        r.finished = true;
        if (r.eval) {
          this.setRunStatus("", "");
          break;
        }
        const ms = typeof ev.ms === "number" ? ` en ${formatMs(ev.ms)}` : "";
        if (ev.ok) {
          this.output.write("ok", `✓ Terminado${ms}`);
          this.setRunStatus(`Terminado${ms}`, "ok");
        } else {
          this.output.write("fail", `✗ Terminado con código ${ev.code}${ms}`);
          this.setRunStatus(`Código ${ev.code}`, "bad");
        }
        break;
      }
    }
  }

  private setRunStatus(text: string, state: string) {
    this.statusRun.textContent = text;
    this.statusRun.dataset.state = state;
    this.titlebar.setStatus(text, state);
  }

  // ---------- Archivos ----------

  newFile() {
    this.editor.newUntitled(SCRIPT_TEMPLATE);
  }

  newExtension() {
    if (settings.formHidden) saveSettings({ formHidden: false });
    this.editor.newUntitled(EXTENSION_TEMPLATE, "acmx");
  }

  /** Añade un @ARG al final de la cabecera de la extensión activa. */
  private async addArg(kind: "text" | "check" | "select" | "list", taken: string[]) {
    const doc = this.editor.active;
    if (!doc) return;
    const line = await newArgDialog(kind, taken);
    if (!line) return;
    this.editor.insertLine(headerInsertLine(this.editor.text(doc)), line);
  }

  private async editMeta(info: { name: string | null; description: string | null; icon: string | null }) {
    const doc = this.editor.active;
    if (!doc) return;
    const meta = await metaDialog({ name: info.name ?? "", description: info.description ?? "", icon: info.icon ?? "" });
    if (!meta) return;
    let src = this.editor.text(doc);
    src = setDirective(src, "@ICON", meta.icon);
    src = setDirective(src, "@DESCRIPTION", meta.description);
    src = setDirective(src, "@EXTENSION", meta.name);
    this.editor.replaceAll(src);
  }

  private async openPath(path: string, quiet = false) {
    const existing = this.editor.find(path, samePath);
    if (existing) {
      this.editor.activate(existing);
      return;
    }
    try {
      this.editor.open(path, await files.read(path));
    } catch (e) {
      if (!quiet) this.output.write("error", String(e));
    }
  }

  private async openFileDialog() {
    const p = await files.pickFile();
    if (p) await this.openPath(p);
  }

  private async openFolder() {
    const p = await files.pickFolder();
    if (!p) return;
    saveSettings({ workspace: p });
    await this.explorer.setRoot(p);
  }

  /** Guarda el documento activo. Devuelve false si se canceló o falló. */
  private async save(as = false): Promise<boolean> {
    const doc = this.editor.active;
    if (!doc) return false;
    return this.saveDoc(doc, as);
  }

  private async saveDoc(doc: OpenDoc, as = false): Promise<boolean> {
    let path = doc.path;
    if (!path || as) {
      const def = settings.workspace ? join(settings.workspace, doc.name) : doc.name;
      path = await files.pickSavePath(def);
      if (!path) return false;
    }
    try {
      await files.write(path, this.editor.text(doc));
      this.editor.markSaved(doc, path);
      if (settings.workspace) await this.explorer.refresh();
      return true;
    } catch (e) {
      this.output.write("error", String(e));
      return false;
    }
  }

  private async closeDoc(doc: OpenDoc) {
    if (this.editor.isDirty(doc)) {
      const choice = await unsavedDialog(doc.name);
      if (choice === "cancel") return;
      if (choice === "save" && !(await this.saveDoc(doc))) return;
    }
    if (this.run?.doc === doc) this.run.doc = null;
    this.editor.close(doc);
  }

  private async confirmCloseAll(): Promise<boolean> {
    for (const doc of this.editor.docs.filter((d) => this.editor.isDirty(d))) {
      this.editor.activate(doc);
      const choice = await unsavedDialog(doc.name);
      if (choice === "cancel") return false;
      if (choice === "save" && !(await this.saveDoc(doc))) return false;
    }
    if (this.run?.id != null) await engine.stop(this.run.id).catch(() => {});
    return true;
  }

  private async newEntry(dir: string, isDir: boolean) {
    const name = await promptText(isDir ? "Nueva carpeta" : "Nuevo archivo", isDir ? "" : "script.acm");
    if (!name) return;
    const path = join(dir, name);
    try {
      await files.create(path, isDir);
      this.explorer.expand(dir);
      await this.explorer.refresh();
      if (!isDir) await this.openPath(path);
    } catch (e) {
      this.output.write("error", String(e));
    }
  }

  private async renameEntry(entry: Entry) {
    const name = await promptText("Renombrar", entry.name);
    if (!name || name === entry.name) return;
    const to = join(dirname(entry.path), name);
    try {
      await files.rename(entry.path, to);
      for (const d of this.editor.docs) {
        if (d.path && isInside(d.path, entry.path)) this.editor.repath(d, to + d.path.slice(entry.path.length));
      }
      await this.explorer.refresh();
      this.refreshChrome();
    } catch (e) {
      this.output.write("error", String(e));
    }
  }

  private async removeEntry(entry: Entry) {
    const ok = await confirmDialog("Eliminar", `¿Eliminar ${entry.isDir ? "la carpeta" : "el archivo"} ${basename(entry.path)}${entry.isDir ? " y todo su contenido" : ""}? No se puede deshacer.`, "Eliminar");
    if (!ok) return;
    try {
      await files.remove(entry.path);
      for (const d of [...this.editor.docs]) if (d.path && isInside(d.path, entry.path)) this.editor.close(d);
      await this.explorer.refresh();
    } catch (e) {
      this.output.write("error", String(e));
    }
  }
}

/** Si `path` es `base` o está dentro de la carpeta `base`. */
function isInside(path: string, base: string): boolean {
  return samePath(path, base) || (path.length > base.length && /[\\/]/.test(path[base.length]) && samePath(path.slice(0, base.length), base));
}

function formatMs(ms: number): string {
  return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(ms < 10000 ? 2 : 1)} s`;
}
