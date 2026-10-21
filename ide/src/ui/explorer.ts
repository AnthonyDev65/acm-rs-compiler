// Explorador de la carpeta de trabajo. Carga las subcarpetas al desplegarlas.

import { basename, dirname, files, type Entry } from "../core/files";
import { button, h, icon, showMenu, type MenuItem } from "./dom";

export interface ExplorerActions {
  open(path: string): void;
  openFolder(): void;
  newEntry(dir: string, isDir: boolean): void;
  rename(entry: Entry): void;
  remove(entry: Entry): void;
  error(message: string): void;
}

export class Explorer {
  readonly el: HTMLElement;
  private title: HTMLElement;
  private tree: HTMLElement;
  private root: string | null = null;
  private expanded = new Set<string>();
  private selected: Entry | null = null;
  private activePath: string | null = null;

  constructor(private actions: ExplorerActions) {
    this.title = h("span", { class: "explorer-title" }, "Sin carpeta");
    const tools = h("div", { class: "explorer-tools" },
      button("fileNew", "Nuevo archivo", () => this.root && actions.newEntry(this.targetDir(), false)),
      button("folderNew", "Nueva carpeta", () => this.root && actions.newEntry(this.targetDir(), true)),
      button("refresh", "Refrescar", () => void this.refresh()),
    );
    this.tree = h("div", { class: "tree", role: "tree" });
    this.el = h("aside", { class: "explorer panel" }, h("header", {}, this.title, tools), this.tree);
  }

  get folder() {
    return this.root;
  }

  async setRoot(path: string | null) {
    this.root = path;
    this.expanded.clear();
    this.selected = null;
    this.title.textContent = path ? basename(path) : "Sin carpeta";
    this.title.title = path ?? "";
    await this.refresh();
  }

  /** Resalta el archivo abierto en el editor. */
  setActive(path: string | null) {
    this.activePath = path;
    for (const row of this.tree.querySelectorAll<HTMLElement>(".row")) {
      row.classList.toggle("active", row.dataset.path === path);
    }
  }

  private targetDir(): string {
    const s = this.selected;
    if (!s) return this.root!;
    return s.isDir ? s.path : dirname(s.path);
  }

  async refresh() {
    if (!this.root) {
      this.tree.replaceChildren(
        h("div", { class: "explorer-empty" },
          h("p", {}, "Abre una carpeta para ver sus scripts."),
          h("button", { class: "btn-text primary", onclick: () => this.actions.openFolder() }, "Abrir carpeta"),
        ),
      );
      return;
    }
    const list = await this.renderDir(this.root, 0);
    this.tree.replaceChildren(list);
    this.setActive(this.activePath);
  }

  private async renderDir(dir: string, depth: number): Promise<HTMLElement> {
    const ul = h("div", { class: "group", role: "group" });
    let entries: Entry[];
    try {
      entries = await files.list(dir);
    } catch (e) {
      this.actions.error(String(e));
      return ul;
    }
    for (const entry of entries) {
      const open = entry.isDir && this.expanded.has(entry.path);
      const row = h("div", {
        class: `row ${entry.isDir ? "dir" : "file"} ${/\.acmx?$/i.test(entry.name) ? "script" : ""}`,
        role: "treeitem",
        tabindex: "-1",
        style: `padding-left:${8 + depth * 12}px`,
        title: entry.path,
      });
      row.dataset.path = entry.path;
      if (entry.isDir) row.setAttribute("aria-expanded", String(open));
      const chev = h("span", { class: `chev ${open ? "open" : ""}` });
      if (entry.isDir) chev.append(icon("chevron"));
      const kind = entry.isDir ? (open ? "folderOpen" : "folder") : /\.acmx$/i.test(entry.name) ? "extension" : /\.acm$/i.test(entry.name) ? "script" : "file";
      row.append(chev, icon(kind), h("span", { class: "name" }, entry.name));
      row.addEventListener("click", () => this.onClick(entry));
      row.addEventListener("contextmenu", (e) => {
        e.preventDefault();
        this.selected = entry;
        this.showMenu(e.clientX, e.clientY, entry);
      });
      ul.append(row);
      if (open) ul.append(await this.renderDir(entry.path, depth + 1));
    }
    return ul;
  }

  private onClick(entry: Entry) {
    this.selected = entry;
    if (entry.isDir) {
      if (this.expanded.has(entry.path)) this.expanded.delete(entry.path);
      else this.expanded.add(entry.path);
      void this.refresh();
    } else {
      this.actions.open(entry.path);
    }
  }

  private showMenu(x: number, y: number, entry: Entry) {
    const dir = entry.isDir ? entry.path : dirname(entry.path);
    const items: (MenuItem | null)[] = [
      { label: "Nuevo archivo…", icon: "fileNew", run: () => this.actions.newEntry(dir, false) },
      { label: "Nueva carpeta…", icon: "folderNew", run: () => this.actions.newEntry(dir, true) },
      null,
      { label: "Renombrar…", icon: "edit", run: () => this.actions.rename(entry) },
      { label: "Eliminar", icon: "trash", danger: true, run: () => this.actions.remove(entry) },
    ];
    if (!entry.isDir) items.unshift({ label: "Abrir", icon: "file", run: () => this.actions.open(entry.path) });
    showMenu({ x, y }, items);
  }

  /** Despliega una carpeta (por ejemplo, tras crear algo dentro). */
  expand(dir: string) {
    if (dir !== this.root) this.expanded.add(dir);
  }
}
