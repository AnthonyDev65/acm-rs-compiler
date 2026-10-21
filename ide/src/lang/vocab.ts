// Vocabulario de acm para resaltado, autocompletado y ayuda. Refleja el README del intérprete.

export interface Doc {
  sig: string;
  info: string;
}

/** Palabras de control de flujo y de bloque. */
export const CONTROL = new Set([
  "IF", "ELSE", "ELSEIF", "ENDIF", "WHILE", "MIENTRAS", "ENDWHILE", "FOR", "FOREACH", "ENDFOR", "NEXT",
  "LOOP", "REPEAT", "ENDLOOP", "INFINITE", "FUNC", "FUNCTION", "DEF", "ENDFUNC", "END", "RETURN",
  "BREAK", "CONTINUE", "EXIT", "QUIT", "SALIR", "IMPORT", "INCLUDE", "VAR", "LET", "SET", "CONST",
  "TO", "STEP", "IN", "THEN",
]);

/** Operadores con nombre y constantes. */
export const OPERATOR_WORDS = new Set(["AND", "OR", "NOT", "EMPTY"]);
export const CONSTANTS = new Set(["TRUE", "FALSE", "NULL"]);
export const PREDEFINED = new Set(["NL", "TAB", "OS", "ARGS", "SCRIPT", "EXITCODE", "LOOP_INDEX"]);

/** Modificadores que aparecen detrás de algunos comandos. */
export const MODIFIERS = new Set([
  "UP", "DOWN", "HORIZONTAL", "LEFT", "RIGHT", "MIDDLE", "ON", "OFF", "CONTINUE", "STOP",
  "CHROME", "EDGE", "BRAVE", "HEADLESS", "IGNORE", "NEW", "CLOSE", "EXISTS", "VISIBLE", "HIDDEN",
  "CLICKABLE", "GONE",
]);

export const KEYS = new Set([
  "ENTER", "SPACE", "TAB", "ESC", "BACKSPACE", "DELETE", "INSERT", "HOME", "END", "PAGEUP", "PAGEDOWN",
  "LEFT", "RIGHT", "UP", "DOWN", "SHIFT", "CTRL", "ALT", "WIN", "META", "CMD", "MOD", "CAPSLOCK",
  "NUMLOCK", "PRINTSCREEN", "PAUSE", "VOLUMEUP", "VOLUMEDOWN", "MUTE", "PLAYPAUSE", "PLUS", "MINUS",
  ...Array.from({ length: 12 }, (_, i) => `F${i + 1}`),
  ...Array.from({ length: 10 }, (_, i) => `NUMPAD${i}`),
]);

export const COMMANDS: Record<string, Doc> = {
  CLICK: { sig: "CLICK x y", info: "Clic izquierdo. Sin coordenadas, clica donde esté el ratón." },
  PRESS: { sig: "PRESS x y [WRITE \"texto\"]", info: "Clic izquierdo; opcionalmente escribe después." },
  DOUBLECLICK: { sig: "DOUBLECLICK x y", info: "Doble clic." },
  RIGHTCLICK: { sig: "RIGHTCLICK x y", info: "Clic derecho." },
  MIDDLECLICK: { sig: "MIDDLECLICK x y", info: "Clic central." },
  MOVE: { sig: "MOVE x y", info: "Mueve el ratón sin clicar." },
  DRAG: { sig: "DRAG x1 y1 TO x2 y2", info: "Arrastra con el botón izquierdo." },
  SCROLL: { sig: "SCROLL [UP|DOWN] n [HORIZONTAL]", info: "Desplaza la rueda (n positivo = hacia abajo)." },
  MOUSEDOWN: { sig: "MOUSEDOWN [LEFT|RIGHT|MIDDLE]", info: "Pulsa un botón del ratón sin soltarlo." },
  MOUSEUP: { sig: "MOUSEUP [LEFT|RIGHT|MIDDLE]", info: "Suelta un botón del ratón." },
  WRITE: { sig: "WRITE expr", info: "Escribe el texto carácter a carácter. Alias: TYPE, TEXT." },
  PASTE: { sig: "PASTE expr", info: "Pega el texto vía portapapeles y restaura el anterior." },
  KEYPRESS: { sig: "KEYPRESS CTRL + SHIFT + T", info: "Pulsa una tecla o combinación. Alias: KEY, HOTKEY." },
  KEYDOWN: { sig: "KEYDOWN tecla", info: "Mantiene una tecla pulsada." },
  KEYUP: { sig: "KEYUP tecla", info: "Suelta una tecla." },
  TYPEDELAY: { sig: "TYPEDELAY ms", info: "Pausa entre caracteres de WRITE (por defecto 10)." },
  DELAY: { sig: "DELAY 500ms | 2s | 1min | expr", info: "Espera. Alias: WAIT, SLEEP." },
  PRINT: { sig: "PRINT a, b, ...", info: "Escribe en la consola. Alias: ECHO, LOG." },
  ASSERT: { sig: "ASSERT cond, \"mensaje\"", info: "Falla (código 1) si la condición es falsa." },
  ASK: { sig: "ASK var \"¿Pregunta?\"", info: "Lee una línea de la consola." },
  OPEN: { sig: "OPEN \"programa | url | archivo\"", info: "Abre un programa, una URL o un archivo." },
  EXEC: { sig: "EXEC \"comando\" [-> var]", info: "Ejecuta un comando del sistema; EXITCODE recoge el código." },
  ALERT: { sig: "ALERT msg", info: "Muestra un diálogo modal y espera." },
  NOTIFY: { sig: "NOTIFY msg [, título]", info: "Muestra una notificación del sistema." },
  WRITEFILE: { sig: "WRITEFILE ruta, contenido", info: "Escribe un archivo." },
  APPENDFILE: { sig: "APPENDFILE ruta, contenido", info: "Añade al final de un archivo." },
  DELETEFILE: { sig: "DELETEFILE ruta", info: "Borra un archivo." },
  MKDIR: { sig: "MKDIR ruta", info: "Crea una carpeta." },
  CLIPSET: { sig: "CLIPSET texto", info: "Copia el texto al portapapeles." },
  FOCUS: { sig: "FOCUS \"título\"", info: "Activa la primera ventana cuyo título o aplicación contiene el texto." },
  WAITWINDOW: { sig: "WAITWINDOW \"título\" [, ms]", info: "Espera a que aparezca una ventana (10 s por defecto)." },
  SCREENSHOT: { sig: "SCREENSHOT \"a.png\" [x y ancho alto]", info: "Captura la pantalla principal o una región." },
  ONERROR: { sig: "ONERROR CONTINUE | STOP", info: "Si un error detiene el script." },
  FAILSAFE: { sig: "FAILSAFE ON | OFF", info: "Parada de emergencia al llevar el ratón a (0,0)." },
  WEBBROWSER: { sig: "WEBBROWSER CHROME|EDGE|BRAVE|\"ruta.exe\" [HEADLESS]", info: "Elige el navegador." },
  WEBPROFILE: { sig: "WEBPROFILE \"carpeta\"", info: "Usa un perfil persistente del navegador." },
  WEBTIMEOUT: { sig: "WEBTIMEOUT ms", info: "Tiempo máximo de espera de los comandos web." },
  WEBOPEN: { sig: "WEBOPEN[(IGNORE)] url", info: "Navega en la pestaña actual y espera a que cargue." },
  WEBTAB: { sig: "WEBTAB [NEW | url | n | CLOSE]", info: "Abre, cambia o cierra pestañas." },
  WEBCLICK: { sig: "WEBCLICK sel", info: "Hace clic en el elemento (CSS o XPath)." },
  WEBINPUT: { sig: "WEBINPUT sel texto", info: "Vacía el campo y escribe el texto." },
  WEBGET: { sig: "WEBGET sel var", info: "Guarda en var el texto visible o el valor del elemento." },
  WEBWAIT: { sig: "WEBWAIT sel [EXISTS|VISIBLE|HIDDEN|CLICKABLE|GONE] [ms]", info: "Espera a que el elemento cumpla el estado." },
  WEBSELECT: { sig: "WEBSELECT sel \"opción\"", info: "Elige una opción de una lista desplegable." },
  WEBCHECK: { sig: "WEBCHECK sel TRUE|FALSE", info: "Marca o desmarca una casilla." },
  WEBSCROLL: { sig: "WEBSCROLL sel", info: "Desplaza la página hasta el elemento." },
  WEBKEY: { sig: "WEBKEY \"Enter\"", info: "Pulsa una tecla dentro de la página." },
  WEBJS: { sig: "WEBJS \"código\" [-> var]", info: "Ejecuta JavaScript en la página." },
  WEBSCREENSHOT: { sig: "WEBSCREENSHOT \"pagina.png\"", info: "Captura la página." },
  WEBBACK: { sig: "WEBBACK", info: "Vuelve a la página anterior." },
  WEBRELOAD: { sig: "WEBRELOAD", info: "Recarga la página." },
  WEBCLOSE: { sig: "WEBCLOSE", info: "Cierra el navegador." },
  WEBWAITCLOSE: { sig: "WEBWAITCLOSE [ms]", info: "Espera a que el usuario cierre el navegador. Útil al final de un script para dejarlo abierto." },
};

/** Alias de comandos que comparten documentación. */
const ALIASES: Record<string, string> = {
  TYPE: "WRITE", TEXT: "WRITE", ESCRIBIR: "WRITE", PEGAR: "PASTE", KEY: "KEYPRESS", HOTKEY: "KEYPRESS",
  PRESSKEY: "KEYPRESS", TECLA: "KEYPRESS", WAIT: "DELAY", SLEEP: "DELAY", ESPERAR: "DELAY",
  ECHO: "PRINT", LOG: "PRINT", IMPRIMIR: "PRINT", INPUT: "ASK", PREGUNTAR: "ASK", ABRIR: "OPEN",
  START: "OPEN", RUN: "EXEC", SHELL: "EXEC", EJECUTAR: "EXEC", ALERTA: "ALERT", NOTIFICAR: "NOTIFY",
  SETCLIPBOARD: "CLIPSET", CLIPBOARD: "CLIPSET", ACTIVATE: "FOCUS", CAPTURE: "SCREENSHOT",
  CAPTURA: "SCREENSHOT", MOUSEMOVE: "MOVE", BROWSER: "WEBBROWSER", WEBGOTO: "WEBOPEN", WEBNAV: "WEBOPEN",
  WEBEVAL: "WEBJS",
};

export function commandDoc(word: string): Doc | undefined {
  const w = word.toUpperCase();
  return COMMANDS[w] ?? (ALIASES[w] ? COMMANDS[ALIASES[w]] : undefined);
}

export function isCommand(word: string): boolean {
  return commandDoc(word) !== undefined;
}

export const FUNCTIONS: Record<string, Doc> = {
  len: { sig: "len(x)", info: "Longitud de un texto o una lista." },
  upper: { sig: "upper(s)", info: "Texto en mayúsculas." },
  lower: { sig: "lower(s)", info: "Texto en minúsculas." },
  trim: { sig: "trim(s)", info: "Quita los espacios de los extremos." },
  replace: { sig: "replace(s, a, b)", info: "Reemplaza a por b." },
  split: { sig: "split(s, sep)", info: "Divide un texto en una lista." },
  join: { sig: "join(lista, sep)", info: "Une una lista en un texto." },
  lines: { sig: "lines(s)", info: "Lista de líneas del texto." },
  contains: { sig: "contains(x, y)", info: "Si el texto o la lista contiene y." },
  startswith: { sig: "startswith(s, p)", info: "Si el texto empieza por p." },
  endswith: { sig: "endswith(s, p)", info: "Si el texto termina en p." },
  indexof: { sig: "indexof(x, y)", info: "Posición de y, o -1." },
  substr: { sig: "substr(s, inicio, largo)", info: "Parte de un texto." },
  pad: { sig: "pad(s, ancho, relleno)", info: "Rellena un texto hasta el ancho." },
  chr: { sig: "chr(n)", info: "Carácter con el código n." },
  ord: { sig: "ord(c)", info: "Código del carácter." },
  num: { sig: "num(x)", info: "Convierte a número." },
  int: { sig: "int(x)", info: "Parte entera." },
  str: { sig: "str(x)", info: "Convierte a texto." },
  isnum: { sig: "isnum(x)", info: "Si x es un número." },
  type: { sig: "type(x)", info: "Nombre del tipo." },
  round: { sig: "round(x, dec)", info: "Redondea." },
  floor: { sig: "floor(x)", info: "Redondea hacia abajo." },
  ceil: { sig: "ceil(x)", info: "Redondea hacia arriba." },
  abs: { sig: "abs(x)", info: "Valor absoluto." },
  min: { sig: "min(a, b, ...)", info: "Mínimo." },
  max: { sig: "max(a, b, ...)", info: "Máximo." },
  sum: { sig: "sum(lista)", info: "Suma." },
  random: { sig: "random() | random(lista) | random(a, b)", info: "Número o elemento aleatorio." },
  randint: { sig: "randint(a, b)", info: "Entero aleatorio entre a y b." },
  range: { sig: "range(n) | range(a, b, paso)", info: "Lista de números." },
  push: { sig: "push(lista, x)", info: "Añade x a la lista." },
  slice: { sig: "slice(lista, a, b)", info: "Parte de una lista." },
  reverse: { sig: "reverse(x)", info: "Invierte una lista o un texto." },
  sort: { sig: "sort(lista)", info: "Ordena." },
  unique: { sig: "unique(lista)", info: "Quita duplicados." },
  json: { sig: "json(x)", info: "Convierte a JSON." },
  parsejson: { sig: "parsejson(s)", info: "Lee un texto JSON." },
  exec: { sig: "exec(\"cmd\")", info: "Salida de un comando del sistema." },
  env: { sig: "env(\"VAR\")", info: "Variable de entorno." },
  exists: { sig: "exists(ruta)", info: "Si existe el archivo o carpeta." },
  read: { sig: "read(ruta)", info: "Contenido de un archivo." },
  readlines: { sig: "readlines(ruta)", info: "Líneas de un archivo." },
  listdir: { sig: "listdir(ruta)", info: "Nombres de los archivos de una carpeta." },
  now: { sig: "now(\"%Y-%m-%d %H:%M:%S\")", info: "Fecha y hora con formato." },
  time: { sig: "time()", info: "Milisegundos desde 1970." },
  mousex: { sig: "mousex()", info: "Posición X del ratón." },
  mousey: { sig: "mousey()", info: "Posición Y del ratón." },
  mousepos: { sig: "mousepos()", info: "Posición del ratón [x, y]." },
  screenwidth: { sig: "screenwidth()", info: "Ancho de la pantalla." },
  screenheight: { sig: "screenheight()", info: "Alto de la pantalla." },
  pixel: { sig: "pixel(x, y)", info: "Color del píxel (#RRGGBB)." },
  clipboard: { sig: "clipboard()", info: "Texto del portapapeles." },
  windows: { sig: "windows()", info: "Títulos de las ventanas abiertas." },
  activewindow: { sig: "activewindow()", info: "Título de la ventana activa." },
  windowexists: { sig: "windowexists(\"texto\")", info: "Si hay una ventana con ese texto." },
  webtext: { sig: "webtext(sel)", info: "Texto del elemento." },
  webexists: { sig: "webexists(sel)", info: "Si el elemento existe." },
  webattr: { sig: "webattr(sel, nombre)", info: "Atributo del elemento." },
  weburl: { sig: "weburl()", info: "URL actual." },
  webtitle: { sig: "webtitle()", info: "Título de la página." },
  webjs: { sig: "webjs(código)", info: "Resultado de un JavaScript." },
  webtabs: { sig: "webtabs()", info: "Pestañas abiertas." },
};

export const DIRECTIVES: Record<string, Doc> = {
  "@EXTENSION": { sig: "@EXTENSION \"nombre\"", info: "Nombre de la extensión." },
  "@DESCRIPTION": { sig: "@DESCRIPTION \"texto\"", info: "Descripción de la extensión." },
  "@AUTHOR": { sig: "@AUTHOR \"nombre\"", info: "Autor de la extensión." },
  "@ICON": { sig: "@ICON \"emoji\"", info: "Icono de la extensión." },
  "@ARG": { sig: "@ARG nombre \"etiqueta\" \"defecto\"", info: "Argumento de texto." },
  "@ARG:CHECK": { sig: "@ARG:CHECK nombre \"etiqueta\" \"false\"", info: "Argumento verdadero/falso." },
  "@ARG:SELECT": { sig: "@ARG:SELECT nombre \"etiqueta\" [\"defecto\"] \"a|b|c\"", info: "Argumento con opciones." },
  "@ARG:LIST": { sig: "@ARG:LIST nombre \"etiqueta\" \"a,b,c\"", info: "Argumento que se convierte en lista." },
};
