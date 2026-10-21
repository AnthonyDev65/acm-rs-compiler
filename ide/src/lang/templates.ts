// Contenido inicial de los archivos nuevos.

export const SCRIPT_TEMPLATE = `// Script acm
PRINT "Hola desde acm"
`;

export const EXTENSION_TEMPLATE = `@EXTENSION "Mi extensión"
@DESCRIPTION "Describe aquí lo que hace la extensión"
@ICON "🧩"
@ARG nombre "Tu nombre" "Ana"
@ARG:CHECK saludar "Mostrar el saludo" "true"
@ARG:SELECT idioma "Idioma" "es|en"

IF saludar
    IF idioma == "es"
        PRINT "Hola, \${nombre}"
    ELSE
        PRINT "Hello, \${nombre}"
    ENDIF
ENDIF
`;

export function isExtension(name: string, source?: string): boolean {
  return /\.acmx$/i.test(name) || (source !== undefined && /^\s*@(EXTENSION|ARG)/im.test(source));
}

/** Línea (1-based) detrás de la cual añadir una directiva nueva: tras la última de la cabecera. */
export function headerInsertLine(source: string): number {
  const lines = source.split("\n");
  let last = 0;
  for (let i = 0; i < lines.length; i++) {
    const t = lines[i].trim();
    if (t.startsWith("@")) last = i + 1;
    else if (t && !t.startsWith("//") && !t.startsWith("#")) break;
  }
  return last;
}

/** Sustituye (o añade) una directiva de cabecera como @EXTENSION "nombre". */
export function setDirective(source: string, directive: string, value: string): string {
  const re = new RegExp(`^([ \\t]*)${directive}\\b.*$`, "im");
  const line = value ? `${directive} "${value.replace(/"/g, "'")}"` : "";
  if (re.test(source)) {
    return value ? source.replace(re, (_m, ind: string) => ind + line) : source.replace(new RegExp(`^[ \\t]*${directive}\\b.*\\r?\\n?`, "im"), "");
  }
  if (!value) return source;
  return `${line}\n${source}`;
}
