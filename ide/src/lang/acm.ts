// Soporte del lenguaje acm para CodeMirror: resaltado, autocompletado, ayuda flotante
// y errores de sintaxis. Los errores los da el propio intérprete (`acm -c`), a través
// de la función `check` que se recibe desde fuera; este módulo no sabe nada de Tauri.

import { autocompletion, type Completion, type CompletionContext } from "@codemirror/autocomplete";
import { HighlightStyle, StreamLanguage, syntaxHighlighting, type StreamParser, type StringStream } from "@codemirror/language";
import { linter, type Diagnostic } from "@codemirror/lint";
import { type Extension } from "@codemirror/state";
import { hoverTooltip, type EditorView } from "@codemirror/view";
import { Tag, tags as t } from "@lezer/highlight";

import { COMMANDS, CONSTANTS, CONTROL, DIRECTIVES, FUNCTIONS, KEYS, MODIFIERS, OPERATOR_WORDS, PREDEFINED, commandDoc, isCommand, type Doc } from "./vocab";

const commandTag = Tag.define();
const interpTag = Tag.define();

interface State {
  /** Dentro de un texto "..." */
  str: boolean;
  /** Profundidad de llaves dentro de ${...} */
  interp: number;
  /** Se acaba de leer FUNC: la siguiente palabra es un nombre de función. */
  afterFunc: boolean;
  /** Ya se leyó la primera palabra de la sentencia. */
  started: boolean;
}

function word(stream: StringStream, st: State): string {
  const w = stream.current();
  const up = w.toUpperCase();
  const first = !st.started;
  st.started = true;
  if (st.afterFunc) {
    st.afterFunc = false;
    return "def";
  }
  if (up === "FUNC" || up === "FUNCTION" || up === "DEF") {
    st.afterFunc = true;
    return "keyword";
  }
  if (CONTROL.has(up)) return "keyword";
  if (OPERATOR_WORDS.has(up)) return "operatorKeyword";
  if (CONSTANTS.has(up)) return "atom";
  if (first && isCommand(w)) return "command";
  if (stream.match(/^\s*\(/, false)) return FUNCTIONS[w.toLowerCase()] ? "builtin" : "function";
  if (isCommand(w) && w === up) return "command";
  if (PREDEFINED.has(w)) return "builtin";
  if ((KEYS.has(w) || MODIFIERS.has(w)) && w === up) return "atom";
  return "variableName";
}

function expression(stream: StringStream, st: State): string | null {
  if (stream.eatSpace()) return null;
  if (stream.match("//")) {
    stream.skipToEnd();
    return "comment";
  }
  const ch = stream.peek()!;
  if (ch === '"') {
    stream.next();
    st.str = true;
    return "string";
  }
  if (ch === "'") {
    stream.next();
    while (!stream.eol() && stream.next() !== "'") {}
    return "string";
  }
  if (stream.match(/^\d+(\.\d+)?(ms|min|s)?\b/i)) return "number";
  if (stream.match(/^[A-Za-z_À-ɏ][\wÀ-ɏ]*/)) return word(stream, st);
  if (st.interp > 0 && ch === "{") {
    stream.next();
    st.interp++;
    return "brace";
  }
  if (st.interp > 0 && ch === "}") {
    stream.next();
    st.interp--;
    if (st.interp === 0) {
      st.str = true;
      return "interp";
    }
    return "brace";
  }
  if (stream.match(/^(->|==|!=|<=|>=|&&|\|\||\+\+|--|\+=|-=|[-+*/%<>=!])/)) return "operator";
  if (stream.match(/^[{}]/)) return "brace";
  if (stream.match(/^[()[\],;.:]/)) {
    if (stream.current() === ";") st.started = false;
    return "punctuation";
  }
  stream.next();
  return null;
}

const parser: StreamParser<State> = {
  name: "acm",
  startState: () => ({ str: false, interp: 0, afterFunc: false, started: false }),
  copyState: (s) => ({ ...s }),
  token(stream, st) {
    if (stream.sol()) {
      // Cada línea es una sentencia nueva; los textos no continúan en la línea siguiente.
      st.str = false;
      st.interp = 0;
      st.started = false;
      st.afterFunc = false;
      if (stream.match(/^\s*#/) || stream.match(/^\s*\/\//)) {
        stream.skipToEnd();
        return "comment";
      }
      if (stream.match(/^\s*@[A-Za-z:]+/)) return "meta";
      // Etiqueta delante de la sentencia: `A:PRESS 100 200`
      if (stream.match(/^\s*[A-Za-z_]\w*:(?=[A-Za-z])/)) return "labelName";
    }
    if (st.str && st.interp === 0) {
      if (stream.match("${")) {
        st.str = false;
        st.interp = 1;
        return "interp";
      }
      while (!stream.eol()) {
        if (stream.match('\\"') || stream.match("\\${")) continue;
        if (stream.match("${", false)) return "string";
        if (stream.next() === '"') {
          st.str = false;
          return "string";
        }
      }
      return "string";
    }
    return expression(stream, st);
  },
  tokenTable: { command: commandTag, interp: interpTag, builtin: t.standard(t.function(t.variableName)), function: t.function(t.variableName) },
  languageData: { commentTokens: { line: "//" } },
};

export const acmLanguage = StreamLanguage.define(parser);

const acmHighlight = HighlightStyle.define([
  { tag: commandTag, color: "var(--syn-command)", fontWeight: "600" },
  { tag: t.keyword, color: "var(--syn-keyword)" },
  { tag: t.operatorKeyword, color: "var(--syn-keyword)" },
  { tag: t.atom, color: "var(--syn-atom)" },
  { tag: t.number, color: "var(--syn-number)" },
  { tag: t.string, color: "var(--syn-string)" },
  { tag: interpTag, color: "var(--syn-interp)" },
  { tag: t.comment, color: "var(--syn-comment)", fontStyle: "italic" },
  { tag: t.meta, color: "var(--syn-meta)" },
  { tag: t.labelName, color: "var(--syn-comment)" },
  { tag: [t.function(t.variableName), t.standard(t.function(t.variableName))], color: "var(--syn-function)" },
  { tag: t.definition(t.variableName), color: "var(--syn-function)", fontWeight: "600" },
  { tag: t.variableName, color: "var(--syn-variable)" },
  { tag: t.operator, color: "var(--syn-operator)" },
]);

// ---------- Autocompletado ----------

function docInfo(d: Doc): () => Node {
  return () => {
    const el = document.createElement("div");
    el.className = "cm-acm-doc";
    el.innerHTML = `<code></code><p></p>`;
    el.querySelector("code")!.textContent = d.sig;
    el.querySelector("p")!.textContent = d.info;
    return el;
  };
}

const staticOptions: Completion[] = [
  ...Object.entries(COMMANDS).map(([label, d]) => ({ label, type: "keyword", detail: "comando", info: docInfo(d), boost: 2 })),
  ...Object.entries(FUNCTIONS).map(([label, d]) => ({ label, type: "function", detail: d.sig, info: docInfo(d), apply: `${label}(` })),
  ...[...CONTROL, ...OPERATOR_WORDS, ...CONSTANTS].map((label) => ({ label, type: "keyword" })),
  ...[...KEYS].map((label) => ({ label, type: "constant", detail: "tecla", boost: -2 })),
  ...[...PREDEFINED].map((label) => ({ label, type: "variable", detail: "predefinida" })),
];

const directiveOptions: Completion[] = Object.entries(DIRECTIVES).map(([label, d]) => ({ label, type: "keyword", info: docInfo(d) }));

/** Variables y funciones declaradas en el propio documento. */
function userSymbols(text: string): Completion[] {
  const out = new Map<string, Completion>();
  for (const m of text.matchAll(/^\s*(?:FUNC|FUNCTION|DEF)\s+([A-Za-z_]\w*)\s*\(([^)]*)\)/gim)) {
    out.set(m[1], { label: m[1], type: "function", detail: `(${m[2].trim()})`, apply: `${m[1]}(` });
  }
  const vars = /(?:^|;)\s*(?:VAR|LET|SET|CONST)\s+([A-Za-z_]\w*)|(?:^|;)\s*([A-Za-z_]\w*)\s*[+-]?=(?!=)|\bFOR\s*\(?\s*(?:VAR\s+)?([A-Za-z_]\w*)|@ARG(?::\w+)?\s+([A-Za-z_]\w*)|(?:ASK|INPUT|WEBGET\s+\S+)\s+([A-Za-z_]\w*)|->\s*([A-Za-z_]\w*)/gim;
  for (const m of text.matchAll(vars)) {
    const name = m.slice(1).find(Boolean);
    if (name && !out.has(name) && !isCommand(name) && !CONTROL.has(name.toUpperCase())) {
      out.set(name, { label: name, type: "variable", boost: 1 });
    }
  }
  return [...out.values()];
}

function complete(ctx: CompletionContext) {
  const dir = ctx.matchBefore(/@[\w:]*/);
  if (dir) return { from: dir.from, options: directiveOptions, validFor: /^@[\w:]*$/ };
  const w = ctx.matchBefore(/[A-Za-z_][\w]*/);
  if (!w && !ctx.explicit) return null;
  // Nada de sugerencias dentro de comentarios o de textos (salvo dentro de ${...}).
  const line = ctx.state.doc.lineAt(ctx.pos);
  const before = line.text.slice(0, ctx.pos - line.from);
  if (/^\s*(#|\/\/)/.test(before)) return null;
  const quotes = (before.replace(/\\"/g, "").match(/"/g) ?? []).length;
  if (quotes % 2 === 1 && before.lastIndexOf("${") <= before.lastIndexOf('"')) return null;
  return {
    from: w ? w.from : ctx.pos,
    options: [...userSymbols(ctx.state.doc.toString()), ...staticOptions],
    validFor: /^\w*$/,
  };
}

// ---------- Ayuda flotante ----------

const hover = hoverTooltip((view, pos) => {
  const { from, to, text } = view.state.doc.lineAt(pos);
  let s = pos - from;
  let e = s;
  while (s > 0 && /[\w@:]/.test(text[s - 1])) s--;
  while (e < text.length && /\w/.test(text[e])) e++;
  const w = text.slice(s, e);
  if (!w) return null;
  const d = w.startsWith("@") ? DIRECTIVES[w.toUpperCase()] : commandDoc(w) ?? FUNCTIONS[w.toLowerCase()];
  if (!d) return null;
  return { pos: from + s, end: Math.min(to, from + e), above: true, create: () => ({ dom: docInfo(d)() as HTMLElement }) };
});

// ---------- Errores de sintaxis ----------

export interface CheckResult {
  ok: boolean;
  line: number | null;
  message: string | null;
}

export type Checker = (source: string) => Promise<CheckResult | null>;

function syntaxLinter(check: Checker) {
  return linter(
    async (view: EditorView): Promise<Diagnostic[]> => {
      const r = await check(view.state.doc.toString());
      if (!r || r.ok) return [];
      const doc = view.state.doc;
      const n = Math.min(Math.max(r.line ?? 1, 1), doc.lines);
      const line = doc.line(n);
      const indent = line.text.length - line.text.trimStart().length;
      return [{ from: line.from + indent, to: line.to, severity: "error", message: r.message ?? "error de sintaxis" }];
    },
    { delay: 500 },
  );
}

/** Todo el soporte del lenguaje. `check` puede devolver null si el intérprete no está disponible. */
export function acm(check: Checker): Extension {
  return [acmLanguage, syntaxHighlighting(acmHighlight), autocompletion({ override: [complete] }), hover, syntaxLinter(check)];
}
