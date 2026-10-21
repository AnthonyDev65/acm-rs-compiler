// Iconos de HugeIcons (paquete gratuito). Cada icono es una lista de elementos SVG
// que aquí se convierten en un <svg> sin depender de ningún framework.

import {
  Add01Icon, AlertCircleIcon, ArrowDown01Icon, ArrowRight01Icon, BrowserIcon, Cancel01Icon, CheckmarkCircle02Icon,
  CheckmarkSquare02Icon, CommandLineIcon, Delete02Icon, Edit02Icon, File01Icon, FileAddIcon, FileCodeIcon,
  FileScriptIcon, FloppyDiskIcon, Folder01Icon, FolderAddIcon, FolderOpenIcon, FormIcon, InputTextIcon, LayoutBottomIcon,
  LeftToRightListBulletIcon, PlayIcon, PuzzleIcon, Refresh01Icon, SidebarLeftIcon, SlidersHorizontalIcon,
  Settings02Icon, StopIcon, TestTube01Icon, ViewIcon,
} from "@hugeicons/core-free-icons";

type IconData = readonly (readonly [string, Record<string, string | number>])[];

// Botones de ventana propios, al estilo de Windows 11: el icono de copiar de HugeIcons
// se leía como «x²» al tamaño de la barra de título.
const line = { stroke: "currentColor", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" };
const WinMinimize: IconData = [["path", { d: "M5 12h14", ...line }]];
const WinMaximize: IconData = [["rect", { x: 5, y: 5, width: 14, height: 14, rx: 2, ...line }]];
const WinRestore: IconData = [
  ["rect", { x: 5, y: 8.5, width: 10.5, height: 10.5, rx: 2, ...line }],
  ["path", { d: "M8.5 5.5A1.5 1.5 0 0 1 10 5h7a2 2 0 0 1 2 2v7a1.5 1.5 0 0 1-.5 1.5", ...line }],
];

const ICONS = {
  play: PlayIcon,
  stop: StopIcon,
  dry: TestTube01Icon,
  folder: Folder01Icon,
  folderOpen: FolderOpenIcon,
  folderNew: FolderAddIcon,
  file: File01Icon,
  fileNew: FileAddIcon,
  script: FileScriptIcon,
  extension: PuzzleIcon,
  code: FileCodeIcon,
  save: FloppyDiskIcon,
  refresh: Refresh01Icon,
  options: SlidersHorizontalIcon,
  settings: Settings02Icon,
  close: Cancel01Icon,
  chevron: ArrowRight01Icon,
  chevronDown: ArrowDown01Icon,
  clear: Delete02Icon,
  trash: Delete02Icon,
  edit: Edit02Icon,
  add: Add01Icon,
  ok: CheckmarkCircle02Icon,
  alert: AlertCircleIcon,
  terminal: CommandLineIcon,
  form: FormIcon,
  preview: ViewIcon,
  sidebar: SidebarLeftIcon,
  panel: LayoutBottomIcon,
  window: BrowserIcon,
  minimize: WinMinimize,
  maximize: WinMaximize,
  restore: WinRestore,
  argText: InputTextIcon,
  argCheck: CheckmarkSquare02Icon,
  argSelect: ArrowDown01Icon,
  argList: LeftToRightListBulletIcon,
} satisfies Record<string, IconData>;

export type IconName = keyof typeof ICONS;

const SVG_NS = "http://www.w3.org/2000/svg";

/** Pasa `strokeWidth` a `stroke-width`, etc. */
function attrName(k: string): string {
  return k.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase());
}

/** Símbolo de acm: el puntero (ratón) y el caret azul (teclado). El caret parpadea mientras se ejecuta un script. */
export function mark(size = 18): SVGSVGElement {
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("class", "acm-mark");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", String(size));
  svg.setAttribute("height", String(size));
  svg.setAttribute("aria-hidden", "true");
  const ptr = document.createElementNS(SVG_NS, "path");
  ptr.setAttribute("d", "M3.75 2.75v14.6l3.9-3.6 2.65 5.95 2.45-1.1-2.65-5.9h5.5z");
  for (const [k, v] of Object.entries({ fill: "currentColor", stroke: "currentColor", "stroke-width": "1.5", "stroke-linejoin": "round" })) ptr.setAttribute(k, v);
  const caret = document.createElementNS(SVG_NS, "rect");
  for (const [k, v] of Object.entries({ class: "acm-caret", x: "15.25", y: "17.75", width: "6", height: "2.75", rx: "0.5" })) caret.setAttribute(k, v);
  svg.append(ptr, caret);
  return svg;
}

export function icon(name: IconName, size = 16): SVGSVGElement {
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("class", "icon");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", String(size));
  svg.setAttribute("height", String(size));
  svg.setAttribute("fill", "none");
  svg.setAttribute("aria-hidden", "true");
  for (const [tag, attrs] of ICONS[name] as IconData) {
    const el = document.createElementNS(SVG_NS, tag);
    for (const [k, v] of Object.entries(attrs)) if (k !== "key") el.setAttribute(attrName(k), String(v));
    svg.append(el);
  }
  return svg;
}
