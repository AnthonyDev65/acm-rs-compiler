import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import { launch } from "./core/launch";
import { App } from "./ui/app";
import { RunnerApp } from "./ui/runner";

const root = document.getElementById("app")!;

// La misma página sirve para el IDE y para la ventana de una extensión .acmx.
if (launch.isRunner()) void new RunnerApp(root).start();
else void new App(root).start();

// Sin el menú contextual del navegador fuera de los campos de texto.
document.addEventListener("contextmenu", (e) => {
  const t = e.target as HTMLElement;
  if (!t.closest("input, textarea, .cm-content")) e.preventDefault();
});
