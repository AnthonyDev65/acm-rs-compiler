// Modo de arranque y asociación de archivos.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const launch = {
  /** La ventana de una extensión se abre con `?runner`. */
  isRunner: () => new URLSearchParams(location.search).has("runner"),
  info: () => invoke<{ file: string | null }>("launch_info"),
  takeOpenRequest: () => invoke<string | null>("take_open_request"),
  openInIde: (path: string) => invoke<void>("open_in_ide", { path }),
  onOpenRequest: (fn: (path: string) => void) => listen<string>("ide://open", (e) => fn(e.payload)),
  acmxAssociated: () => invoke<boolean>("acmx_associated"),
  associateAcmx: () => invoke<void>("associate_acmx"),
};
