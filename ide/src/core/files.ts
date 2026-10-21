// Acceso a archivos y diálogos del sistema.

import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

export interface Entry {
  name: string;
  path: string;
  isDir: boolean;
}

const SCRIPT_FILTERS = [
  { name: "Scripts acm", extensions: ["acm", "acmx"] },
  { name: "Todos los archivos", extensions: ["*"] },
];

export const files = {
  list: (path: string) => invoke<Entry[]>("fs_list", { path }),
  read: (path: string) => invoke<string>("fs_read", { path }),
  write: (path: string, content: string) => invoke<void>("fs_write", { path, content }),
  create: (path: string, dir: boolean) => invoke<void>("fs_create", { path, dir }),
  rename: (from: string, to: string) => invoke<void>("fs_rename", { from, to }),
  remove: (path: string) => invoke<void>("fs_delete", { path }),

  pickFolder: async () => (await open({ directory: true })) as string | null,
  pickFile: async () => (await open({ filters: SCRIPT_FILTERS })) as string | null,
  pickSavePath: (defaultPath?: string) => save({ filters: SCRIPT_FILTERS, defaultPath }),
};

const SEP = /[\\/]/;

export function basename(p: string): string {
  return p.split(SEP).pop() ?? p;
}

export function dirname(p: string): string {
  const i = Math.max(p.lastIndexOf("\\"), p.lastIndexOf("/"));
  return i > 0 ? p.slice(0, i) : p;
}

export function join(dir: string, name: string): string {
  const sep = dir.includes("\\") ? "\\" : "/";
  return dir.endsWith(sep) ? dir + name : dir + sep + name;
}

export function samePath(a: string, b: string): boolean {
  return a.replace(/\\/g, "/").toLowerCase() === b.replace(/\\/g, "/").toLowerCase();
}
