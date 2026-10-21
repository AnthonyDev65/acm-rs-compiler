// Panel de salida: muestra lo que emite el intérprete y tiene una línea de entrada
// que sirve de consola (evaluar código) o para responder a ASK.

import { button, h, icon } from "./dom";

export type LineKind = "print" | "info" | "error" | "warn" | "dry" | "step" | "stderr" | "ok" | "fail" | "input";

const MAX_LINES = 5000;

export class OutputPanel {
  readonly el: HTMLElement;
  private log: HTMLElement;
  private input: HTMLInputElement;
  private prompt: HTMLElement;
  private history: string[] = [];
  private histPos = 0;

  onSubmit: (text: string) => void = () => {};
  onGoto: (line: number) => void = () => {};

  constructor() {
    this.log = h("div", { class: "log", role: "log", "aria-live": "polite" });
    this.prompt = h("label", { class: "prompt", for: "console-input" }, "acm>");
    this.input = h("input", { id: "console-input", type: "text", spellcheck: "false", autocomplete: "off" });
    this.setPrompt(null);
    this.input.addEventListener("keydown", (e) => this.onKey(e));
    const header = h("header", {},
      icon("terminal", 14),
      h("span", { class: "panel-title" }, "Salida"),
      h("div", { class: "spacer" }),
      button("clear", "Limpiar salida", () => this.clear()),
    );
    this.el = h("section", { class: "output panel" }, header, this.log, h("div", { class: "input-row" }, this.prompt, this.input));
  }

  private onKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      const text = this.input.value;
      if (!text.trim() && !this.asking) return;
      if (text.trim()) {
        this.history.push(text);
        this.histPos = this.history.length;
      }
      this.input.value = "";
      this.onSubmit(text);
    } else if (e.key === "ArrowUp" && this.history.length) {
      e.preventDefault();
      this.histPos = Math.max(0, this.histPos - 1);
      this.input.value = this.history[this.histPos];
    } else if (e.key === "ArrowDown" && this.history.length) {
      e.preventDefault();
      this.histPos = Math.min(this.history.length, this.histPos + 1);
      this.input.value = this.history[this.histPos] ?? "";
    }
  }

  private asking = false;

  /** null = modo consola; un texto = pregunta de ASK pendiente. */
  setPrompt(question: string | null) {
    this.asking = question !== null;
    this.prompt.textContent = question ?? "acm>";
    this.prompt.classList.toggle("asking", this.asking);
    this.input.placeholder = this.asking ? "Escribe la respuesta y pulsa Enter" : "Evalúa código acm: PRINT mousex(), mousey()";
    if (this.asking) this.input.focus();
  }

  write(kind: LineKind, text: string, line?: number | null) {
    const row = h("div", { class: `line ${kind}` });
    if (line != null) {
      row.append(h("button", { class: "goto", title: "Ir a la línea", onclick: () => this.onGoto(line) }, `línea ${line}`), " ");
    }
    row.append(text);
    const stick = this.log.scrollTop + this.log.clientHeight >= this.log.scrollHeight - 8;
    this.log.append(row);
    while (this.log.childElementCount > MAX_LINES) this.log.firstElementChild!.remove();
    if (stick) this.log.scrollTop = this.log.scrollHeight;
  }

  separator() {
    if (this.log.childElementCount) this.log.append(h("hr", {}));
  }

  clear() {
    this.log.replaceChildren();
  }

  focusInput() {
    this.input.focus();
  }
}
