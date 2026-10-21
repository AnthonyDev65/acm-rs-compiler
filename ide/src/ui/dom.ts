// Utilidades mínimas para construir la interfaz sin framework.

import { icon, mark, type IconName } from "./icons";

export { icon, mark, type IconName };

type Child = Node | string | null | undefined | false;
type Attrs = Record<string, string | number | boolean | EventListener | null | undefined>;

export function h<K extends keyof HTMLElementTagNameMap>(tag: K, attrs: Attrs = {}, ...children: Child[]): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v == null || v === false) continue;
    if (k.startsWith("on") && typeof v === "function") el.addEventListener(k.slice(2), v);
    else el.setAttribute(k, v === true ? "" : String(v));
  }
  for (const c of children) if (c) el.append(c);
  return el;
}

export function button(iconName: IconName, label: string, onClick: (e: MouseEvent) => void, opts: { text?: string; cls?: string; kbd?: string } = {}): HTMLButtonElement {
  const b = h("button", { type: "button", class: `btn ${opts.cls ?? ""}`, title: opts.kbd ? `${label} (${opts.kbd})` : label, "aria-label": label });
  b.addEventListener("click", (e) => onClick(e));
  b.append(icon(iconName));
  if (opts.text) b.append(h("span", {}, opts.text));
  return b;
}

export interface MenuItem {
  label: string;
  icon?: IconName;
  hint?: string;
  danger?: boolean;
  run: () => void;
}

let openMenu: HTMLElement | null = null;

export function closeMenu() {
  openMenu?.remove();
  openMenu = null;
}

document.addEventListener("mousedown", (e) => {
  if (openMenu && !openMenu.contains(e.target as Node)) closeMenu();
});
document.addEventListener("keydown", (e) => e.key === "Escape" && closeMenu());
window.addEventListener("blur", closeMenu);

/** Menú flotante en (x, y) o bajo un elemento. `null` en la lista dibuja un separador. */
export function showMenu(at: { x: number; y: number } | HTMLElement, items: (MenuItem | null)[]) {
  closeMenu();
  const menu = h("div", { class: "menu", role: "menu" });
  for (const it of items) {
    if (!it) {
      menu.append(h("hr", {}));
      continue;
    }
    const b = h("button", { type: "button", class: `menu-item ${it.danger ? "danger" : ""}`, role: "menuitem" });
    b.append(it.icon ? icon(it.icon) : h("span", { class: "icon-gap" }), h("span", { class: "label" }, it.label));
    if (it.hint) b.append(h("kbd", {}, it.hint));
    b.addEventListener("click", () => {
      closeMenu();
      it.run();
    });
    menu.append(b);
  }
  document.body.append(menu);
  let x: number;
  let y: number;
  if (at instanceof HTMLElement) {
    const r = at.getBoundingClientRect();
    x = r.left;
    y = r.bottom + 6;
  } else {
    ({ x, y } = at);
  }
  const mw = menu.offsetWidth;
  const mh = menu.offsetHeight;
  menu.style.left = `${Math.min(x, window.innerWidth - mw - 8)}px`;
  menu.style.top = `${Math.min(y, window.innerHeight - mh - 8)}px`;
  openMenu = menu;
  (menu.querySelector("button") as HTMLElement | null)?.focus();
}
