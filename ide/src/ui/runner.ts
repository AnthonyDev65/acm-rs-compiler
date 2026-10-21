// Ventana de una extensión .acmx abierta desde el Explorador: muestra sus @ARG como
// formulario, la ejecuta con acm y enseña el resultado. No necesita el IDE abierto.

import { engine, type ExtensionInfo, type RunMessage } from "../core/engine";
import { basename, dirname, files } from "../core/files";
import { launch } from "../core/launch";
import { saveSettings, settings } from "../core/settings";
import { appWindow } from "../core/window";
import { argForm, windowHeader, type ArgForm } from "./argform";
import { h, icon } from "./dom";

type State = "idle" | "running" | "done";

export class RunnerApp {
  private file: string | null = null;
  private info: ExtensionInfo | null = null;
  private form: ArgForm | null = null;
  private state: State = "idle";
  private runId: number | null = null;
  private early: RunMessage[] = [];
  private pendingRun = false;
  private finished = false;

  private head = h("div", { class: "runner-head", "data-tauri-drag-region": true });
  private body = h("div", { class: "runner-body" });
  private log = h("div", { class: "runner-log", role: "log", hidden: true });
  private askRow = h("div", { class: "runner-ask", hidden: true });
  private chip = h("span", { class: "run-chip", hidden: true });
  private footer = h("footer", { class: "runner-foot" });
  private minimize = h("input", { type: "checkbox", class: "switch", id: "runner-min" });

  constructor(private root: HTMLElement) {
    document.body.classList.add("runner-mode");
    this.minimize.checked = settings.runnerMinimize;
    this.minimize.addEventListener("change", () => saveSettings({ runnerMinimize: this.minimize.checked }));
    const controls = h("div", { class: "runner-controls" },
      this.chip,
      h("button", { type: "button", class: "win-btn", "aria-label": "Minimizar", onclick: () => void appWindow.minimize() }, icon("minimize", 15)),
      h("button", { type: "button", class: "win-btn close", "aria-label": "Cerrar", onclick: () => void appWindow.close() }, icon("close", 15)),
    );
    this.root.append(
      h("div", { class: "runner" },
        h("header", { class: "runner-title", "data-tauri-drag-region": true }, this.head, controls),
        h("div", { class: "runner-scroll" }, this.body, this.log, this.askRow),
        this.footer,
      ),
    );
    window.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && this.state !== "running") void appWindow.close();
    });
    appWindow.onCloseRequested(async () => {
      if (this.runId !== null && this.state === "running") await engine.stop(this.runId).catch(() => {});
      return true;
    });
  }

  async start() {
    void engine.onMessage((m) => this.onMessage(m));
    const { file } = await launch.info();
    this.file = file;
    if (!file) return this.fail("No se indicó ninguna extensión.");
    try {
      await engine.locate(settings.acmPath || undefined);
    } catch (e) {
      return this.fail(String(e));
    }
    let source: string;
    try {
      source = await files.read(file);
      this.info = await engine.info(source);
    } catch (e) {
      return this.fail(String(e));
    }
    const name = this.info.name ?? basename(file).replace(/\.acmx?$/i, "");
    void appWindow.setTitle(name);
    this.head.replaceChildren(windowHeader(this.info, name));
    this.form = argForm(this.info, settings.argValues[file] ?? {}, () => this.saveValues());
    this.body.replaceChildren(this.form.el);
    this.renderFooter();
    (this.body.querySelector("input, select") as HTMLElement | null)?.focus();
  }

  private fail(message: string) {
    this.head.replaceChildren(h("div", { class: "ext-titlebar" }, h("span", { class: "ext-glyph bad" }, icon("alert", 16)), h("span", { class: "ext-name" }, "No se pudo abrir la extensión")));
    this.body.replaceChildren(h("p", { class: "runner-error" }, message));
    this.footer.replaceChildren(h("div", { class: "spacer" }), h("button", { type: "button", class: "btn-text", onclick: () => void appWindow.close() }, "Cerrar"));
  }

  private saveValues() {
    if (this.file && this.form) saveSettings({ argValues: { ...settings.argValues, [this.file]: this.form.read() } });
  }

  private renderFooter() {
    const edit = h("button", { type: "button", class: "btn-text ghost", title: "Abrir en acm IDE", onclick: () => this.file && void launch.openInIde(this.file) }, icon("code", 14), "Editar");
    const minLabel = h("label", { class: "runner-min", for: "runner-min", title: "Minimiza esta ventana mientras se ejecuta para no tapar la automatización" }, this.minimize, "Ocultar al ejecutar");
    let actions: HTMLElement[];
    if (this.state === "running") {
      actions = [h("button", { type: "button", class: "btn-text primary danger", onclick: () => void this.stop() }, icon("stop", 14), "Detener")];
    } else {
      const run = h("button", { type: "submit", class: "btn-text primary", onclick: () => void this.run() }, icon("play", 14), this.state === "done" ? "Ejecutar de nuevo" : "Ejecutar");
      actions = [h("button", { type: "button", class: "btn-text", onclick: () => void appWindow.close() }, this.state === "done" ? "Cerrar" : "Cancelar"), run];
    }
    this.footer.replaceChildren(edit, minLabel, h("div", { class: "spacer" }), ...actions);
  }

  private setChip(text: string, state: string) {
    this.chip.hidden = !text;
    this.chip.textContent = text;
    this.chip.dataset.state = state;
  }

  private write(kind: string, text: string) {
    this.log.hidden = false;
    this.log.append(h("div", { class: `line ${kind}` }, text));
    this.log.scrollTop = this.log.scrollHeight;
  }

  private async run() {
    if (!this.file || !this.form || this.state === "running") return;
    const values = this.form.read();
    this.saveValues();
    this.state = "running";
    this.finished = false;
    this.runId = null;
    this.early = [];
    this.pendingRun = true;
    this.log.replaceChildren();
    this.log.hidden = true;
    this.setChip("Ejecutando…", "busy");
    this.renderFooter();
    try {
      const id = await engine.run({
        path: this.file,
        cwd: dirname(this.file),
        args: Object.entries(values).map(([k, v]) => `${k}=${v}`),
        noFailsafe: settings.noFailsafe,
      });
      this.runId = id;
      this.pendingRun = false;
      const early = this.early.filter((m) => m.runId === id);
      this.early = [];
      early.forEach((m) => this.onMessage(m));
      if (settings.runnerMinimize && this.state === "running") void appWindow.minimize();
    } catch (e) {
      this.pendingRun = false;
      this.write("error", String(e));
      this.finish(false, "Error");
    }
  }

  private async stop() {
    if (this.runId !== null) await engine.stop(this.runId).catch((e) => this.write("error", String(e)));
  }

  private onMessage(m: RunMessage) {
    if (this.pendingRun) {
      this.early.push(m);
      return;
    }
    if (m.runId !== this.runId) return;
    if (m.kind === "stdout") this.write("print", m.text);
    else if (m.kind === "stderr") this.write("stderr", m.text);
    else if (m.kind === "exit") {
      if (!this.finished) {
        this.write("fail", "■ Detenido");
        this.finish(false, "Detenido");
      }
      this.askRow.hidden = true;
    } else if (m.kind === "event") {
      const ev = m.event as Record<string, any>;
      switch (ev.event) {
        case "print":
          this.write("print", String(ev.text));
          break;
        case "warning":
          this.write("warn", String(ev.message));
          break;
        case "error":
          this.write("error", ev.line != null ? `Línea ${ev.line}: ${ev.message}` : String(ev.message));
          break;
        case "screenshot":
          this.write("info", `Captura guardada: ${ev.path}`);
          break;
        case "ask":
          this.ask(String(ev.prompt || `${ev.var}:`));
          break;
        case "done":
          this.finished = true;
          if (ev.ok) {
            this.write("ok", `✓ Terminado${typeof ev.ms === "number" ? ` en ${(ev.ms / 1000).toFixed(1)} s` : ""}`);
            this.finish(true, "Terminado");
          } else {
            this.write("fail", `✗ Terminó con código ${ev.code}`);
            this.finish(false, `Código ${ev.code}`);
          }
          break;
      }
    }
  }

  /** El script pide un dato con ASK: se muestra la ventana y un campo para responder. */
  private ask(prompt: string) {
    void appWindow.restore();
    const input = h("input", { type: "text", spellcheck: "false" });
    const send = () => {
      if (this.runId === null) return;
      this.write("input", `${prompt} ${input.value}`);
      void engine.input(this.runId, input.value);
      this.askRow.hidden = true;
    };
    input.addEventListener("keydown", (e) => e.key === "Enter" && send());
    this.askRow.replaceChildren(
      h("label", {}, prompt),
      h("div", { class: "runner-ask-row" }, input, h("button", { type: "button", class: "btn-text primary", onclick: send }, "Enviar")),
    );
    this.askRow.hidden = false;
    input.focus();
  }

  private finish(ok: boolean, label: string) {
    this.state = "done";
    this.setChip(label, ok ? "ok" : "bad");
    this.renderFooter();
    void appWindow.restore();
  }
}
