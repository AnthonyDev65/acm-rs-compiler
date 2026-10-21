// Área de edición: pestañas y una sola vista de CodeMirror que cambia de estado
// según el documento activo (cada documento conserva su historial de deshacer).

import { closeBrackets, closeBracketsKeymap, completionKeymap } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab, toggleComment } from "@codemirror/commands";
import { bracketMatching, indentOnInput, indentUnit } from "@codemirror/language";
import { lintKeymap } from "@codemirror/lint";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import { EditorState, StateEffect, StateField, type Extension, type Text } from "@codemirror/state";
import {
  Decoration, drawSelection, EditorView, highlightActiveLine, highlightActiveLineGutter, highlightSpecialChars,
  keymap, lineNumbers, rectangularSelection, type DecorationSet,
} from "@codemirror/view";

import { basename } from "../core/files";
import { h, icon } from "./dom";

export interface OpenDoc {
  id: number;
  path: string | null;
  name: string;
  state: EditorState;
  saved: Text;
}

// ---------- Marcas de ejecución: línea actual y línea con error ----------

const setRunLine = StateEffect.define<number | null>();
const setErrorLine = StateEffect.define<number | null>();

const runMarks = StateField.define<{ run: number | null; error: number | null; deco: DecorationSet }>({
  create: () => ({ run: null, error: null, deco: Decoration.none }),
  update(v, tr) {
    let { run, error } = v;
    for (const e of tr.effects) {
      if (e.is(setRunLine)) run = e.value;
      if (e.is(setErrorLine)) error = e.value;
    }
    if (tr.docChanged) error = null;
    if (run === v.run && error === v.error && !tr.docChanged) return v;
    const doc = tr.state.doc;
    const marks = [];
    for (const [n, cls] of [[run, "cm-runLine"], [error, "cm-errorLine"]] as const) {
      if (n != null && n >= 1 && n <= doc.lines) marks.push(Decoration.line({ class: cls }).range(doc.line(n).from));
    }
    marks.sort((a, b) => a.from - b.from);
    return { run, error, deco: Decoration.set(marks) };
  },
  provide: (f) => EditorView.decorations.from(f, (v) => v.deco),
});

export class EditorArea {
  readonly el: HTMLElement;
  private tabs: HTMLElement;
  private host: HTMLElement;
  private empty: HTMLElement;
  private view: EditorView;
  private extensions: Extension;
  private nextId = 1;
  private untitled = 1;
  docs: OpenDoc[] = [];
  active: OpenDoc | null = null;

  /** Cambió el documento activo, la lista de pestañas o el estado de guardado. */
  onChange: () => void = () => {};
  onCursor: (line: number, col: number) => void = () => {};

  constructor(language: Extension, empty: HTMLElement) {
    this.tabs = h("div", { class: "tabs", role: "tablist" });
    this.host = h("div", { class: "editor-host" });
    this.empty = empty;
    this.el = h("section", { class: "editor-area panel" }, this.tabs, this.host, this.empty);
    this.extensions = [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightSpecialChars(),
      history(),
      drawSelection(),
      EditorState.allowMultipleSelections.of(true),
      indentOnInput(),
      indentUnit.of("    "),
      bracketMatching(),
      closeBrackets(),
      rectangularSelection(),
      highlightActiveLine(),
      highlightSelectionMatches(),
      keymap.of([
        ...closeBracketsKeymap, ...defaultKeymap, ...searchKeymap, ...historyKeymap, ...completionKeymap, ...lintKeymap,
        indentWithTab, { key: "Mod-/", run: toggleComment },
      ]),
      language,
      runMarks,
      EditorView.updateListener.of((u) => {
        if (u.docChanged) this.onChange();
        if (u.selectionSet || u.docChanged) this.reportCursor(u.state);
      }),
    ];
    this.view = new EditorView({ parent: this.host, state: EditorState.create({ extensions: this.extensions }) });
    this.render();
  }

  private reportCursor(state: EditorState) {
    const pos = state.selection.main.head;
    const line = state.doc.lineAt(pos);
    this.onCursor(line.number, pos - line.from + 1);
  }

  private sync() {
    if (this.active) this.active.state = this.view.state;
  }

  private create(path: string | null, content: string, name: string): OpenDoc {
    const state = EditorState.create({ doc: content, extensions: this.extensions });
    const doc: OpenDoc = { id: this.nextId++, path, name, state, saved: state.doc };
    this.docs.push(doc);
    this.activate(doc);
    return doc;
  }

  open(path: string, content: string): OpenDoc {
    return this.create(path, content, basename(path));
  }

  newUntitled(content = "", ext = "acm"): OpenDoc {
    return this.create(null, content, `sin título ${this.untitled++}.${ext}`);
  }

  /** Inserta una línea completa detrás de la línea `after` (0 = al principio) del documento activo. */
  insertLine(after: number, text: string) {
    const doc = this.view.state.doc;
    const pos = after <= 0 ? 0 : doc.line(Math.min(after, doc.lines)).to;
    const insert = after <= 0 ? `${text}\n` : `\n${text}`;
    this.view.dispatch({ changes: { from: pos, insert }, selection: { anchor: pos + (after <= 0 ? 0 : 1) }, scrollIntoView: true });
  }

  /** Sustituye todo el texto del documento activo (se puede deshacer). */
  replaceAll(text: string) {
    this.view.dispatch({ changes: { from: 0, to: this.view.state.doc.length, insert: text } });
  }

  find(path: string, eq: (a: string, b: string) => boolean): OpenDoc | undefined {
    return this.docs.find((d) => d.path && eq(d.path, path));
  }

  activate(doc: OpenDoc) {
    this.sync();
    this.active = doc;
    this.view.setState(doc.state);
    this.reportCursor(doc.state);
    this.render();
    this.onChange();
    this.view.focus();
  }

  close(doc: OpenDoc) {
    this.sync();
    const i = this.docs.indexOf(doc);
    if (i < 0) return;
    this.docs.splice(i, 1);
    if (this.active === doc) {
      this.active = null;
      const next = this.docs[Math.min(i, this.docs.length - 1)];
      if (next) this.activate(next);
    }
    this.render();
    this.onChange();
  }

  text(doc: OpenDoc): string {
    return (doc === this.active ? this.view.state : doc.state).doc.toString();
  }

  isDirty(doc: OpenDoc): boolean {
    return !(doc === this.active ? this.view.state : doc.state).doc.eq(doc.saved);
  }

  markSaved(doc: OpenDoc, path?: string) {
    this.sync();
    doc.saved = doc.state.doc;
    if (path) {
      doc.path = path;
      doc.name = basename(path);
    }
    this.render();
    this.onChange();
  }

  /** Cambia la ruta de un documento abierto (al renombrar en el explorador). */
  repath(doc: OpenDoc, path: string) {
    doc.path = path;
    doc.name = basename(path);
    this.render();
  }

  setRunLine(doc: OpenDoc | null, line: number | null) {
    if (doc && doc === this.active) this.view.dispatch({ effects: setRunLine.of(line) });
    else if (doc) doc.state = doc.state.update({ effects: setRunLine.of(line) }).state;
  }

  setErrorLine(doc: OpenDoc | null, line: number | null) {
    if (!doc) return;
    if (doc !== this.active) {
      doc.state = doc.state.update({ effects: setErrorLine.of(line) }).state;
      return;
    }
    const effects: StateEffect<unknown>[] = [setErrorLine.of(line)];
    if (line != null && line >= 1 && line <= this.view.state.doc.lines) {
      effects.push(EditorView.scrollIntoView(this.view.state.doc.line(line).from, { y: "center" }));
    }
    this.view.dispatch({ effects });
  }

  gotoLine(line: number) {
    const doc = this.view.state.doc;
    const l = doc.line(Math.min(Math.max(line, 1), doc.lines));
    this.view.dispatch({ selection: { anchor: l.from }, effects: EditorView.scrollIntoView(l.from, { y: "center" }) });
    this.view.focus();
  }

  focus() {
    this.view.focus();
  }

  render() {
    this.tabs.replaceChildren(
      ...this.docs.map((d) => {
        const dirty = this.isDirty(d);
        const close = h("button", { class: "tab-close", title: "Cerrar (Ctrl+W)", "aria-label": `Cerrar ${d.name}` });
        close.append(icon("close", 12));
        close.addEventListener("click", (e) => {
          e.stopPropagation();
          this.onCloseRequest(d);
        });
        const tab = h("div", {
          class: `tab ${d === this.active ? "active" : ""} ${dirty ? "dirty" : ""}`,
          role: "tab",
          "aria-selected": String(d === this.active),
          title: d.path ?? d.name,
          onmousedown: (e: Event) => {
            if ((e as MouseEvent).button === 1) {
              e.preventDefault();
              this.onCloseRequest(d);
            } else if (d !== this.active) this.activate(d);
          },
        }, icon(/\.acmx$/i.test(d.name) ? "extension" : "script", 14), h("span", { class: "tab-name" }, d.name), close);
        return tab;
      }),
    );
    const none = this.docs.length === 0;
    this.host.hidden = none;
    this.empty.hidden = !none;
  }

  /** La aplicación decide si hay que preguntar antes de cerrar. */
  onCloseRequest: (doc: OpenDoc) => void = (doc) => this.close(doc);
}
