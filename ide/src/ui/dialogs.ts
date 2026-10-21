// Diálogos modales propios (el webview no garantiza prompt/confirm nativos).

import type { EngineInfo, ExtensionArg, ExtensionInfo } from "../core/engine";
import { ARG_ICONS, argForm, windowHeader } from "./argform";
import { h, icon } from "./dom";

interface ModalButton<T> {
  label: string;
  primary?: boolean;
  danger?: boolean;
  value: () => T | undefined;
}

function modal<T>(head: string | HTMLElement, body: HTMLElement, buttons: ModalButton<T>[], cancelValue: T, cls = ""): Promise<T> {
  return new Promise((resolve) => {
    const prev = document.activeElement as HTMLElement | null;
    const finish = (v: T) => {
      overlay.classList.add("leaving");
      setTimeout(() => overlay.remove(), 120);
      prev?.focus();
      resolve(v);
    };
    const footer = h("footer", {});
    for (const b of buttons) {
      footer.append(
        h("button", {
          class: `btn-text ${b.primary ? "primary" : ""} ${b.danger ? "danger" : ""}`,
          type: b.primary ? "submit" : "button",
          onclick: (e: Event) => {
            e.preventDefault();
            const v = b.value();
            if (v !== undefined) finish(v);
          },
        }, b.label),
      );
    }
    const close = h("button", { type: "button", class: "dialog-close", "aria-label": "Cerrar", onclick: () => finish(cancelValue) }, icon("close", 14));
    const header = typeof head === "string" ? h("header", { class: "dialog-head" }, h("h2", {}, head), close) : h("header", { class: "dialog-head custom" }, head, close);
    const form = h("form", { class: `dialog ${cls}`, role: "dialog", "aria-modal": "true" }, header, body, footer);
    const overlay = h("div", { class: "overlay" }, form);
    overlay.addEventListener("mousedown", (e) => e.target === overlay && finish(cancelValue));
    form.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        finish(cancelValue);
      }
    });
    document.body.append(overlay);
    const first = form.querySelector<HTMLElement>(".dialog-body input, .dialog-body select, footer .primary");
    first?.focus();
    if (first instanceof HTMLInputElement && first.type === "text") first.select();
  });
}

function textField(label: string, value = "", placeholder = "", hint = ""): { row: HTMLElement; input: HTMLInputElement } {
  const input = h("input", { type: "text", value, placeholder, spellcheck: "false" });
  const row = h("label", { class: "field" }, h("span", {}, label), input, hint ? h("small", {}, hint) : null);
  return { row, input };
}

export function promptText(title: string, value = "", placeholder = ""): Promise<string | null> {
  const f = textField("Nombre", value, placeholder);
  return modal(title, h("div", { class: "dialog-body" }, f.row), [
    { label: "Cancelar", value: () => null },
    { label: "Aceptar", primary: true, value: () => f.input.value.trim() || undefined },
  ], null);
}

export function confirmDialog(title: string, message: string, ok = "Aceptar"): Promise<boolean> {
  return modal(title, h("div", { class: "dialog-body" }, h("p", { class: "message" }, message)), [
    { label: "Cancelar", value: () => false },
    { label: ok, primary: true, danger: true, value: () => true },
  ], false);
}

/** Pregunta qué hacer con cambios sin guardar. */
export function unsavedDialog(name: string): Promise<"save" | "discard" | "cancel"> {
  return modal<"save" | "discard" | "cancel">("Cambios sin guardar", h("div", { class: "dialog-body" }, h("p", { class: "message" }, `¿Quieres guardar los cambios de ${name}?`)), [
    { label: "No guardar", value: () => "discard" },
    { label: "Cancelar", value: () => "cancel" },
    { label: "Guardar", primary: true, value: () => "save" },
  ], "cancel");
}

/** La ventana de una extensión .acmx: sus @ARG como formulario. Devuelve los valores o null. */
export function argsDialog(info: ExtensionInfo, fallbackName: string, saved: Record<string, string>): Promise<Record<string, string> | null> {
  const form = argForm(info, saved);
  return modal<Record<string, string> | null>(windowHeader(info, fallbackName), h("div", { class: "dialog-body" }, form.el), [
    { label: "Cancelar", value: () => null },
    { label: "Ejecutar", primary: true, value: () => form.read() },
  ], null, "ext-window");
}

const KIND_LABEL: Record<ExtensionArg["type"], string> = {
  text: "Texto",
  check: "Casilla (sí/no)",
  select: "Lista de opciones",
  list: "Lista de valores",
};

const quote = (s: string) => `"${s.replace(/"/g, "'")}"`;

/** Pide los datos de un campo nuevo y devuelve la línea @ARG correspondiente. */
export function newArgDialog(kind: ExtensionArg["type"], taken: string[]): Promise<string | null> {
  const name = textField("Nombre de la variable", "", "mi_valor", "Así se usa en el script: ${mi_valor}");
  const label = textField("Etiqueta", "", "Texto que verá el usuario");
  const def = textField(kind === "check" ? "Marcada por defecto" : "Valor por defecto", "", kind === "list" ? "a, b, c" : "");
  const options = textField("Opciones", "", "rapido|lento|normal", "Separadas por |. La primera es la opción por defecto.");
  const check = h("input", { type: "checkbox", class: "switch" });
  const error = h("p", { class: "message bad" });
  const rows: HTMLElement[] = [name.row, label.row];
  if (kind === "check") rows.push(h("label", { class: "field inline" }, h("span", {}, "Marcada por defecto"), check));
  else if (kind === "select") rows.push(options.row);
  else rows.push(def.row);
  const head = h("div", { class: "kind-head" }, h("span", { class: "kind-icon" }, icon(ARG_ICONS[kind], 16)), h("h2", {}, `Nuevo campo · ${KIND_LABEL[kind]}`));
  return modal<string | null>(head, h("div", { class: "dialog-body" }, ...rows, error), [
    { label: "Cancelar", value: () => null },
    {
      label: "Añadir",
      primary: true,
      value: () => {
        const n = name.input.value.trim();
        if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(n)) {
          error.textContent = "El nombre solo puede tener letras, números y _, y no puede empezar por un número.";
          return undefined;
        }
        if (taken.some((t) => t.toLowerCase() === n.toLowerCase())) {
          error.textContent = `Ya existe un campo llamado ${n}.`;
          return undefined;
        }
        const l = quote(label.input.value.trim() || n);
        switch (kind) {
          case "check":
            return `@ARG:CHECK ${n} ${l} "${check.checked}"`;
          case "select": {
            const opts = options.input.value.split("|").map((o) => o.trim()).filter(Boolean);
            if (!opts.length) {
              error.textContent = "Escribe al menos una opción.";
              return undefined;
            }
            return `@ARG:SELECT ${n} ${l} ${quote(opts.join("|"))}`;
          }
          case "list":
            return `@ARG:LIST ${n} ${l} ${quote(def.input.value.trim())}`;
          default:
            return `@ARG ${n} ${l} ${quote(def.input.value)}`;
        }
      },
    },
  ], null);
}

export interface ExtensionMeta {
  name: string;
  description: string;
  icon: string;
}

/** Nombre, descripción e icono de la extensión. */
export function metaDialog(current: ExtensionMeta): Promise<ExtensionMeta | null> {
  const name = textField("Nombre", current.name, "Mi extensión");
  const desc = textField("Descripción", current.description, "Qué hace la extensión");
  const glyph = textField("Icono", current.icon, "🧩", "Un emoji.");
  return modal<ExtensionMeta | null>("Datos de la extensión", h("div", { class: "dialog-body" }, name.row, desc.row, glyph.row), [
    { label: "Cancelar", value: () => null },
    { label: "Guardar", primary: true, value: () => ({ name: name.input.value.trim(), description: desc.input.value.trim(), icon: glyph.input.value.trim() }) },
  ], null);
}

/** Ajustes del intérprete. `detect` prueba una ruta (vacía = automática). */
export function settingsDialog(
  current: string,
  detect: (p: string) => Promise<EngineInfo>,
  assoc: { check: () => Promise<boolean>; associate: () => Promise<void> },
): Promise<string | null> {
  const assocStatus = h("p", { class: "message small" });
  const assocBtn = h("button", { type: "button", class: "btn-text" }, "Asociar .acmx");
  const refreshAssoc = async () => {
    const ok = await assoc.check().catch(() => false);
    assocStatus.textContent = ok ? "Los .acmx se abren en su propia ventana." : "Los .acmx todavía no se abren con acm IDE.";
    assocStatus.className = `message small ${ok ? "ok" : ""}`;
    assocBtn.hidden = ok;
  };
  assocBtn.addEventListener("click", async () => {
    try {
      await assoc.associate();
    } catch (e) {
      assocStatus.textContent = String(e);
      assocStatus.className = "message small bad";
      return;
    }
    await refreshAssoc();
  });
  void refreshAssoc();
  const assocRow = h("div", { class: "field" }, h("span", {}, "Doble clic en un .acmx"), h("div", { class: "actions" }, assocBtn, assocStatus));
  const f = textField("Ruta del intérprete acm", current, "Detectar automáticamente", "Déjala vacía para buscarlo en el PATH y en ~/.cargo/bin.");
  const status = h("p", { class: "message small" });
  const test = async () => {
    status.textContent = "Comprobando…";
    status.className = "message small";
    try {
      const i = await detect(f.input.value.trim());
      status.textContent = `${i.version} · ${i.path}`;
      status.classList.add("ok");
    } catch (e) {
      status.textContent = String(e);
      status.classList.add("bad");
    }
  };
  void test();
  return modal<string | null>("Ajustes", h("div", { class: "dialog-body" }, f.row, h("div", { class: "actions" }, h("button", { type: "button", class: "btn-text", onclick: test }, "Probar"), status), assocRow), [
    { label: "Cancelar", value: () => null },
    { label: "Guardar", primary: true, value: () => f.input.value.trim() },
  ], null);
}
