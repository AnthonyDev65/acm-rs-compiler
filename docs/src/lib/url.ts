// Rutas internas respetando `base` (por ejemplo, GitHub Pages en /acm-rs-compiler/).
const base = import.meta.env.BASE_URL.replace(/\/$/, "");
export const u = (path: string) => `${base}${path}`;
export const REPO = "https://github.com/AnthonyDev65/acm-rs-compiler";
