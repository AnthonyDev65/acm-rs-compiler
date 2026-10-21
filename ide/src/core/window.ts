// Ventana nativa: título, controles de la barra propia y cierre con confirmación.

import { getCurrentWindow } from "@tauri-apps/api/window";

const win = getCurrentWindow();

export const appWindow = {
  setTitle: (title: string) => win.setTitle(title).catch(() => {}),
  minimize: () => win.minimize(),
  restore: async () => {
    await win.unminimize().catch(() => {});
    await win.setFocus().catch(() => {});
  },
  toggleMaximize: () => win.toggleMaximize(),
  close: () => win.close(),
  isMaximized: () => win.isMaximized(),
  onResized: (fn: () => void) => win.onResized(fn),
  /** `canClose` decide si la ventana puede cerrarse (por ejemplo, tras preguntar por cambios sin guardar). */
  onCloseRequested: (canClose: () => Promise<boolean>) =>
    win.onCloseRequested(async (e) => {
      if (!(await canClose())) e.preventDefault();
    }),
};
