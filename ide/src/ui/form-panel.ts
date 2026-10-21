// Panel lateral de las extensiones .acmx: muestra en vivo la ventana con los @ARG
// tal como la verá quien ejecute la extensión, y permite añadir campos sin escribirlos.

import type { ExtensionArg, ExtensionInfo } from "../core/engine";
import { ARG_ICONS, argForm, windowHeader, type ArgForm } from "./argform";
import { button, h, icon, showMenu } from "./dom";

export interface FormPanelActions {
  /** Lee los @ARG del código actual (a través del intérprete). */
  info(source: string): Promise<ExtensionInfo>;
  saved(): Record<string, string>;
  save(values: Record<string, string>): void;
  run(values: Record<string, string>): void;
  addArg(kind: ExtensionArg["type"], taken: string[]): void;
  editMeta(info: ExtensionInfo): void;
}

export class FormPanel {
  readonly el: HTMLElement;
  private body: HTMLElement;
  private runBtn: HTMLButtonElement;
  private form: ArgForm | null = null;
  private info: ExtensionInfo | null = null;
  private name = "";
  private pending = 0;
  private lastSource: string | null = null;

  constructor(private actions: FormPanelActions) {
    const add = button("add", "Añadir campo", (e) => this.addMenu(e.currentTarget as HTMLElement), { text: "Campo", cls: "soft" });
    const meta = button("edit", "Nombre, descripción e icono", () => this.info && this.actions.editMeta(this.info));
    this.body = h("div", { class: "form-body" });
    this.runBtn = h("button", { type: "button", class: "btn-text primary block" }, icon("play", 14), "Ejecutar con estos valores");
    this.runBtn.addEventListener("click", () => this.form && this.actions.run(this.form.read()));
    this.el = h("aside", { class: "form-panel panel" },
      h("header", {}, h("span", { class: "panel-title" }, "Ventana de la extensión"), h("div", { class: "spacer" }), meta, add),
      this.body,
      h("footer", {}, this.runBtn),
    );
  }

  private addMenu(anchor: HTMLElement) {
    const taken = this.info?.args.map((a) => a.name) ?? [];
    const item = (kind: ExtensionArg["type"], label: string, hint: string) => ({ label, hint, icon: ARG_ICONS[kind], run: () => this.actions.addArg(kind, taken) });
    showMenu(anchor, [
      item("text", "Texto", "@ARG"),
      item("check", "Casilla", "CHECK"),
      item("select", "Opciones", "SELECT"),
      item("list", "Lista de valores", "LIST"),
    ]);
  }

  /** Actualiza la vista previa con el código del documento (con un pequeño retardo). */
  update(source: string, name: string, immediate = false) {
    if (source === this.lastSource && name === this.name) return;
    this.name = name;
    const ticket = ++this.pending;
    const go = async () => {
      if (ticket !== this.pending) return;
      this.lastSource = source;
      try {
        const info = await this.actions.info(source);
        if (ticket === this.pending) this.render(info);
      } catch (e) {
        if (ticket === this.pending) this.renderError(String(e));
      }
    };
    if (immediate) void go();
    else setTimeout(go, 350);
  }

  private render(info: ExtensionInfo) {
    // Conserva lo que el usuario haya escrito en la vista previa.
    const values = { ...this.actions.saved(), ...(this.form?.read() ?? {}) };
    this.info = info;
    this.form = argForm(info, values, () => this.form && this.actions.save(this.form.read()));
    const win = h("div", { class: "ext-preview" },
      windowHeader(info, this.name, h("span", { class: "ext-dots" }, h("i", {}), h("i", {}), h("i", {}))),
      h("div", { class: "ext-preview-body" }, this.form.el),
    );
    const legend = h("p", { class: "form-hint" }, `${info.args.length} ${info.args.length === 1 ? "campo" : "campos"} · se actualiza al editar las líneas @ARG`);
    this.body.replaceChildren(win, legend);
    this.runBtn.disabled = false;
  }

  private renderError(msg: string) {
    this.info = null;
    this.form = null;
    this.body.replaceChildren(h("p", { class: "form-hint bad" }, msg));
    this.runBtn.disabled = true;
  }

  reset() {
    this.lastSource = null;
    this.form = null;
    this.info = null;
  }
}
