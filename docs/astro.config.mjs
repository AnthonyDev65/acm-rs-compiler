import { defineConfig } from "astro/config";
import { acmGrammar } from "./src/lib/acm-grammar.mjs";

// La documentación se genera desde el README.md de la raíz del repositorio.
export default defineConfig({
  site: "https://anthonydev65.github.io",
  base: process.env.ACM_DOCS_BASE ?? "/",
  markdown: {
    shikiConfig: {
      theme: "github-dark-default",
      langs: [acmGrammar],
    },
  },
  vite: {
    server: { fs: { allow: [".."] } },
  },
});
