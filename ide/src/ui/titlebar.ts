// Barra de título propia: sustituye la de Windows y reúne archivo, ejecución y controles de ventana.

import { appWindow } from "../core/window";
import { button, h, icon, mark, showMenu, type MenuItem } from "./dom";

export interface TitlebarActions {
  newScript(): void;
  newExtension(): void;
  openFile(): void;
  openFolder(): void;
  save(): void;
  saveAs(): void;
  run(): void;
  dry(): void;
  stop(): void;
  settings(): void;
  toggleSidebar(): void;
  toggleForm(): void;
  /** Opciones de ejecución con su estado actual. */
  options(): { label: string; checked: boolean; toggle: () => void }[];
}

export class Titlebar {
  readonly el: HTMLElement;
  private docName = h("span", { class: "doc-name" });
  private docIcon = h("span", { class: "doc-icon" });
  private status = h("span", { class: "run-chip", hidden: true });
  private runBtn: HTMLButtonElement;
  private dryBtn: HTMLButtonElement;
  private stopBtn: HTMLButtonElement;
  private formBtn: HTMLButtonElement;
  private maxBtn: HTMLButtonElement;

  constructor(actions: TitlebarActions) {
    const fileMenu = (anchor: HTMLElement) =>
      showMenu(anchor, [
        { label: "Nuevo script", icon: "script", hint: "Ctrl+N", run: actions.newScript },
        { label: "Nueva extensión (.acmx)", icon: "extension", hint: "Ctrl+Shift+N", run: actions.newExtension },
        null,
        { label: "Abrir archivo…", icon: "file", hint: "Ctrl+O", run: actions.openFile },
        { label: "Abrir carpeta…", icon: "folderOpen", run: actions.openFolder },
        null,
        { label: "Guardar", icon: "save", hint: "Ctrl+S", run: actions.save },
        { label: "Guardar como…", icon: "save", hint: "Ctrl+Shift+S", run: actions.saveAs },
      ]);
    const optionsMenu = (anchor: HTMLElement) =>
      showMenu(anchor, actions.options().map<MenuItem>((o) => ({ label: o.label, icon: o.checked ? "ok" : undefined, run: o.toggle })));

    this.runBtn = button("play", "Ejecutar", () => actions.run(), { text: "Ejecutar", cls: "run", kbd: "F5" });
    this.dryBtn = button("dry", "Simular sin mover nada", () => actions.dry(), { cls: "dry", kbd: "Ctrl+F5" });
    this.stopBtn = button("stop", "Detener", () => actions.stop(), { cls: "stop", kbd: "Shift+F5" });
    this.formBtn = button("form", "Ventana de la extensión", () => actions.toggleForm());
    this.maxBtn = h("button", { type: "button", class: "win-btn", "aria-label": "Maximizar", onclick: () => void appWindow.toggleMaximize() });

    const left = h("div", { class: "tb-left" },
      h("div", { class: "logo", "data-tauri-drag-region": true }, mark(18)),
      button("sidebar", "Mostrar u ocultar el explorador", () => actions.toggleSidebar()),
      button("fileNew", "Archivo", (e) => fileMenu(e.currentTarget as HTMLElement), { text: "Archivo", cls: "soft" }),
    );
    const center = h("div", { class: "tb-center", "data-tauri-drag-region": true },
      h("div", { class: "doc-pill", "data-tauri-drag-region": true }, this.docIcon, this.docName, this.status),
    );
    const runGroup = h("div", { class: "run-group" }, this.runBtn, this.dryBtn, this.stopBtn);
    const right = h("div", { class: "tb-right" },
      runGroup,
      button("options", "Opciones de ejecución", (e) => optionsMenu(e.currentTarget as HTMLElement)),
      this.formBtn,
      button("settings", "Ajustes", () => actions.settings()),
      h("div", { class: "win-controls" },
        h("button", { type: "button", class: "win-btn", "aria-label": "Minimizar", onclick: () => void appWindow.minimize() }, icon("minimize", 16)),
        this.maxBtn,
        h("button", { type: "button", class: "win-btn close", "aria-label": "Cerrar", onclick: () => void appWindow.close() }, icon("close", 16)),
      ),
    );

    this.el = h("header", { class: "titlebar", "data-tauri-drag-region": true }, left, center, right);
    void this.syncMax();
    void appWindow.onResized(() => void this.syncMax());
  }

  private async syncMax() {
    const max = await appWindow.isMaximized().catch(() => false);
    this.maxBtn.replaceChildren(icon(max ? "restore" : "maximize", 16));
    this.maxBtn.setAttribute("aria-label", max ? "Restaurar" : "Maximizar");
    document.body.classList.toggle("maximized", max);
  }

  setDoc(name: string | null, dirty: boolean, extension: boolean) {
    this.docIcon.replaceChildren(name ? icon(extension ? "extension" : "script", 14) : "");
    this.docName.textContent = name ?? "acm IDE";
    this.docName.classList.toggle("dirty", dirty);
    this.formBtn.hidden = !extension;
  }

  setRunState(canRun: boolean, running: boolean) {
    this.runBtn.disabled = !canRun;
    this.dryBtn.disabled = !canRun;
    this.stopBtn.disabled = !running;
    this.el.classList.toggle("running", running);
  }

  setStatus(text: string, state: string) {
    this.status.hidden = !text;
    this.status.textContent = text;
    this.status.dataset.state = state;
  }

  setFormActive(on: boolean) {
    this.formBtn.classList.toggle("on", on);
  }
}
