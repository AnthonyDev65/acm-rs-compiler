// Campos de los @ARG de una extensión .acmx. Se usa en la vista previa del panel
// y en la ventana que aparece al ejecutar la extensión.

import type { ExtensionArg, ExtensionInfo } from "../core/engine";
import { h, icon, type IconName } from "./dom";

export const ARG_ICONS: Record<ExtensionArg["type"], IconName> = {
  text: "argText",
  check: "argCheck",
  select: "argSelect",
  list: "argList",
};

let uid = 0;

function field(a: ExtensionArg, current: string, onInput: () => void): { row: HTMLElement; read: () => string } {
  const id = `arg-${++uid}`;
  let read: () => string;
  let control: HTMLElement;
  if (a.type === "check") {
    const c = h("input", { type: "checkbox", id, class: "switch" });
    c.checked = current.toLowerCase() === "true";
    c.addEventListener("change", onInput);
    read = () => String(c.checked);
    const row = h("label", { class: "arg-row check", for: id }, h("span", { class: "arg-label" }, a.label || a.name, h("code", {}, a.name)), c);
    return { row, read };
  }
  if (a.type === "select") {
    const s = h("select", { id }, ...a.options.map((o) => h("option", { value: o }, o)));
    s.value = a.options.includes(current) ? current : a.options[0] ?? "";
    s.addEventListener("change", onInput);
    read = () => s.value;
    control = h("div", { class: "select-wrap" }, s, icon("chevronDown", 14));
  } else {
    const i = h("input", { type: "text", id, value: current, spellcheck: "false", placeholder: a.type === "list" ? "valor1, valor2, valor3" : "" });
    i.addEventListener("input", onInput);
    read = () => i.value;
    control = i;
  }
  const label = h("label", { class: "arg-label", for: id }, a.label || a.name, h("code", {}, a.name));
  if (a.type === "list") label.append(h("small", {}, "separados por comas"));
  return { row: h("div", { class: "arg-row" }, label, control), read };
}

export interface ArgForm {
  el: HTMLElement;
  read(): Record<string, string>;
}

/** Construye el formulario. `onInput` se llama cada vez que el usuario cambia un valor. */
export function argForm(info: ExtensionInfo, saved: Record<string, string>, onInput: () => void = () => {}): ArgForm {
  const fields = info.args.map((a) => ({ a, ...field(a, saved[a.name] ?? a.default, onInput) }));
  const el = h("div", { class: "arg-form" },
    info.description ? h("p", { class: "arg-desc" }, info.description) : null,
    ...fields.map((f) => f.row),
    fields.length ? null : h("p", { class: "arg-empty" }, "Esta extensión no tiene argumentos."),
  );
  return { el, read: () => Object.fromEntries(fields.map((f) => [f.a.name, f.read()])) };
}

/** Cabecera estilo ventana con el icono y el nombre de la extensión. */
export function windowHeader(info: ExtensionInfo, fallback: string, extra?: Node): HTMLElement {
  const glyph = info.icon ? h("span", { class: "ext-glyph" }, info.icon) : h("span", { class: "ext-glyph" }, icon("extension", 16));
  return h("div", { class: "ext-titlebar" }, glyph, h("span", { class: "ext-name" }, info.name ?? fallback), extra ?? null);
}
