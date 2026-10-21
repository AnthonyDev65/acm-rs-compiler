# acm

`acm` es un lenguaje de automatización y su intérprete de línea de comandos, escrito en Rust. Controla el ratón, el teclado, las ventanas, el portapapeles, las capturas de pantalla, los procesos y el navegador (Chrome/Edge) con una sintaxis de alto nivel. Se usa igual que `python`:

```bash
acm script.acm                 # ejecutar un archivo
acm -e 'CLICK 500 300; WRITE "hola"'   # ejecutar código suelto
acm                            # consola interactiva (REPL)
```

Sirve para automatizaciones personales y para que **agentes de IA** controlen el PC y hagan pruebas. Ofrece salida JSON, modo simulación (`--dry-run`) y códigos de salida fiables.

Los scripts usan la extensión `.acm`. Las extensiones con argumentos configurables usan `.acmx`.

![El IDE de acm ejecuta un script que abre un perfil de GitHub en el navegador y muestra un ALERT](https://raw.githubusercontent.com/AnthonyDev65/acm-rs-compiler/main/docs/public/demo.gif)

*El IDE ejecutando [`examples/github.acm`](https://github.com/AnthonyDev65/acm-rs-compiler/blob/main/examples/github.acm): abre un perfil de GitHub y avisa con un `ALERT`.*

---

## Instalación

No necesitas Rust. Ejecuta una sola línea:

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/AnthonyDev65/acm-rs-compiler/main/install.ps1 | iex
```

**macOS / Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/AnthonyDev65/acm-rs-compiler/main/install.sh | sh
```

El instalador descarga el binario de la última [Release](https://github.com/AnthonyDev65/acm-rs-compiler/releases) y lo deja en el PATH (`%LOCALAPPDATA%\Programs\acm\bin` en Windows, `~/.local/bin` en macOS/Linux). Para actualizar, vuelve a ejecutar la misma línea; para una versión concreta, define antes `ACM_VERSION` (por ejemplo `v0.1.0`). Para desinstalar, borra esa carpeta o el archivo `acm`.

**Desde el código fuente** (necesitas Rust, <https://rustup.rs>): `cargo install --path .` desde esta carpeta; se desinstala con `cargo uninstall acm`.

**Windows, opcional:** para poder abrir los `.acm` con doble clic o ejecutarlos como `script.acm` en la terminal, ejecuta `acm --register`.

**macOS/Linux:** empieza el script con `#!/usr/bin/env acm` y dale permisos con `chmod +x`.

- **macOS:** concede a la terminal los permisos de *Accesibilidad* y *Grabación de pantalla*.
- **Linux (X11):** ALERT necesita `zenity` o `kdialog`, y FOCUS necesita `wmctrl` o `xdotool`.

La web necesita Google Chrome, Edge o Brave instalados. No hace falta ningún WebDriver.

---

## Línea de comandos

```
acm [OPCIONES] [ARCHIVO] [nombre=valor ...]
```

| Opción | Efecto |
|---|---|
| `-e, --eval CÓDIGO` | Ejecuta código en vez de un archivo. Las expresiones sueltas muestran su valor. El código va entre comillas simples o dobles y funciona igual en cmd, PowerShell y bash: `acm -e 'WRITE "hola"'`. |
| `-` como archivo | Lee el script desde stdin (`echo 'PRINT 1' \| acm -`). |
| `-c, --check` | Solo comprueba la sintaxis. |
| `--info` | Muestra los metadatos y los argumentos `@ARG` de una extensión. |
| `--json` | Emite eventos JSON, uno por línea (ver abajo). |
| `-v, --verbose` | Muestra cada sentencia antes de ejecutarla. |
| `--dry-run` | Simula: muestra lo que haría sin mover el ratón, teclear ni abrir nada. |
| `-k, --keep-going` | Sigue aunque un comando falle (equivale a `ONERROR CONTINUE`). |
| `--no-dialogs` | ALERT y NOTIFY se escriben en la consola en vez de abrir ventanas. |
| `--no-failsafe` | Desactiva la parada de emergencia. |
| `--ask` | Pregunta por consola los `@ARG` que no se hayan pasado. |
| `-i, --interactive` | Abre la consola interactiva al terminar el script, con sus variables cargadas. |
| `--register` | (Windows) Asocia `.acm` y `.acmx` con `acm`. |

Los argumentos `nombre=valor` se convierten en variables del script. Respetan el tipo declarado con `@ARG` en las extensiones. Los argumentos que no siguen el formato `nombre=valor` van a la lista `ARGS`.

**Códigos de salida:**

| Código | Significado |
|---|---|
| `0` | Todo correcto |
| `1` | Error en tiempo de ejecución |
| `2` | Error de sintaxis o de argumentos |
| `130` | Cancelado con Ctrl+C o por FAILSAFE |
| `N` | El script ejecutó `EXIT N` |

**Parada de emergencia (FAILSAFE):** si llevas el ratón a la esquina superior izquierda (0,0), el script se detiene antes de la siguiente acción de ratón o teclado. Ctrl+C también lo cancela.

### Uso desde agentes

```bash
acm --json --no-dialogs script.acm
```

Con `--json`, cada línea de la salida es un evento:

```json
{"event":"start","script":"script.acm","statements":12}
{"event":"print","text":"..."}
{"event":"screenshot","path":"C:\\...\\captura.png"}
{"event":"error","line":7,"message":"no se encontró el elemento '#login' (esperé 10 s)"}
{"event":"done","ok":false,"code":1,"ms":5321}
```

Con `-v` también se emiten eventos `step` (`{"event":"step","line":5,"code":"CLICK 100 200"}`). Con `--dry-run` se emiten eventos `dry` con cada acción simulada.

Ejemplos de consultas rápidas:

```bash
acm -e 'PRINT json(windows())'           # lista de ventanas
acm -e 'PRINT mousex(), mousey()'        # posición del ratón
acm -e 'SCREENSHOT "pantalla.png"'       # captura completa
acm -e 'PRINT pixel(100, 200)'           # color de un píxel (#RRGGBB)
```

---

## El lenguaje

- Una sentencia por línea. Puedes poner varias en la misma línea separadas por `;`.
- Los **comandos** no distinguen mayúsculas (`click` = `CLICK`). Las **variables** sí.
- Los comentarios empiezan con `//` o `#`. También se permite `//` al final de una línea.
- La indentación es libre.
- Una sentencia puede llevar una etiqueta delante, que se ignora al ejecutar: `A:PRESS 100 200`.
- Si un comando termina en ` (N)`, se repite N veces, con 50 ms entre repeticiones: `KEYPRESS TAB (3)`, `WRITE "=" (40)`.

### Valores y expresiones

| Tipo | Ejemplo |
|---|---|
| Número | `10`, `2.5` |
| Texto | `"hola ${nombre}"` interpola. `'texto literal'` no interpola. |
| Booleano | `TRUE`, `FALSE` |
| Nulo | `NULL` |
| Lista | `["a", "b", 3]` |

Para interpolar, usa cualquier expresión dentro de `${...}`: `"${i * 2}"`, `"${ips[i]}"`, `"${upper(x)}"`, `"${RANDOM("a","b")}"`.

- Si la variable no existe o el contenido no es una expresión válida (por ejemplo, código JavaScript), el texto `${...}` se deja tal cual.
- Para escribir `${` literal, usa `\${`.
- Dentro de `"..."` solo se escapa `\"`. Las rutas `"C:\carpeta\archivo"` funcionan sin escapar nada.
- Hay variables predefinidas: `NL` (salto de línea), `TAB`, `OS` (`windows`/`macos`/`linux`), `ARGS`, `SCRIPT`, `EXITCODE` (código del último EXEC) y `LOOP_INDEX`.

**Operadores, de menor a mayor precedencia:**

1. `OR` / `||`
2. `AND` / `&&`
3. `NOT`
4. `==` `!=` `<` `<=` `>` `>=`
5. `+` `-`
6. `*` `/` `%`
7. Unarios: `-` `!` `EMPTY`
8. Acceso: `x[i]`, `x.length`, `x.metodo(args)`, que equivale a `metodo(x, args)`

- `+` suma si los dos lados son números, aunque sean textos numéricos como `"5" + 1` → `6`. En otro caso concatena: `"10.0.0." + 5` → `"10.0.0.5"`.
- `"=" * 10` repite el texto.
- Las comparaciones son numéricas cuando ambos lados son números y de texto en otro caso.
- `EMPTY x` es verdadero si `x` no existe, está vacía o es una lista vacía. `!EMPTY x` es lo contrario.
- Índices negativos: `lista[-1]` es el último elemento.

### Variables y funciones

```acm
VAR nombre = "Ana"          // también LET / SET
VAR ips = ["10.0.0.1", "10.0.0.2"]
contador = 0
contador += 1
contador++
ips[0] = "10.0.0.99"

FUNC saludo(quien, veces)
    FOR i = 1 TO veces
        PRINT "Hola ${quien}"
    NEXT
    RETURN veces * 2
ENDFUNC
VAR r = saludo("Ana", 2)

IMPORT "utilidades.acm"     // ejecuta otro script (ruta relativa al actual)
```

Dentro de una función, `VAR` crea una variable local. Una asignación sin `VAR` modifica la variable global si existe.

### Control de flujo

```acm
IF (x > 5 AND nombre == "Ana")
    PRINT "sí"
ELSE IF x == 0          // también ELSEIF
    PRINT "cero"
ELSE
    PRINT "no"
ENDIF

FOR (VAR i = 0; i < ips.length; i++)    // estilo C (también con i--, i += 2)
    PRINT ips[i]
ENDFOR

FOR ip IN ips           // listas; un número N recorre 0..N-1; "a,b,c" se separa por comas
    PRINT ip
ENDFOR

FOR i = 10 TO 1 STEP -1 // estilo BASIC (cierre con NEXT o ENDFOR)
    PRINT i
NEXT

WHILE NOT windowexists("Bloc de notas")
    DELAY 200ms
ENDWHILE

LOOP 3                  // LOOP / LOOP INFINITE; cierre con ENDLOOP
    KEYPRESS F5
ENDLOOP

BREAK / CONTINUE / RETURN valor / EXIT código
```

- Los bloques también pueden cerrarse con `END` o `}`, y abrirse con `{` al final de la línea.
- Un `LOOP` sin `ENDLOOP` repite todo el resto del script.

### Ratón y teclado

| Comando | Descripción |
|---|---|
| `CLICK x y` / `PRESS x y` | Clic izquierdo. Sin coordenadas, clica donde esté el ratón. |
| `PRESS x y WRITE "texto"` | Clic y luego escribe el texto. |
| `DOUBLECLICK x y`, `RIGHTCLICK x y`, `MIDDLECLICK x y` | Doble clic, clic derecho y clic central. |
| `MOVE x y` | Mueve el ratón sin clicar. |
| `DRAG x1 y1 TO x2 y2` | Arrastra con el botón izquierdo. |
| `SCROLL n` / `SCROLL UP n` / `SCROLL DOWN n` / `SCROLL n HORIZONTAL` | Desplaza la rueda (n positivo = hacia abajo). |
| `MOUSEDOWN [LEFT\|RIGHT\|MIDDLE]`, `MOUSEUP` | Pulsa o suelta un botón. |
| `WRITE expr` (también `TYPE`, `TEXT`) | Escribe el texto carácter a carácter (Unicode). |
| `PASTE expr` | Pega el texto vía portapapeles (rápido) y restaura el portapapeles anterior. |
| `KEYPRESS CTRL + SHIFT + T` (también `KEY`, `HOTKEY`) | Pulsa una tecla o combinación. `CTRL+C` sin espacios también vale. |
| `KEYDOWN SHIFT` / `KEYUP SHIFT` | Mantiene una tecla pulsada o la suelta. |
| `TYPEDELAY ms` | Pausa entre caracteres de WRITE (por defecto 10). |
| `DELAY 500ms` / `DELAY 2s` / `DELAY 1min` / `DELAY expr` | Espera. Alias: `WAIT`, `SLEEP`. |

**Nombres de tecla:**

| Grupo | Nombres |
|---|---|
| Básicas | `ENTER`, `SPACE`, `TAB`, `ESC`, `BACKSPACE`, `DELETE`, `INSERT` |
| Navegación | `HOME`, `END`, `PAGEUP`, `PAGEDOWN`, `LEFT`, `RIGHT`, `UP`, `DOWN` |
| Modificadores | `SHIFT`, `CTRL`, `ALT`, `WIN` (alias `META`, `CMD`), `MOD` (CMD en macOS y CTRL en el resto) |
| Función y teclado numérico | `F1`–`F12`, `NUMPAD0`–`NUMPAD9` |
| Bloqueos y sistema | `CAPSLOCK`, `NUMLOCK`, `PRINTSCREEN`, `PAUSE` |
| Multimedia | `VOLUMEUP`, `VOLUMEDOWN`, `MUTE`, `PLAYPAUSE` |
| Signos | `PLUS` (+), `MINUS` (-) |
| Caracteres | Cualquier carácter suelto (`a`, `1`, `/`) |

Un nombre que no es una tecla se busca como variable: `VAR t = "F5"; KEYPRESS t`.

### Sistema, archivos y ventanas

| Comando | Descripción |
|---|---|
| `PRINT a, b, ...` | Escribe en la consola (stdout). Alias: `ECHO`, `LOG`. |
| `ASSERT cond, "mensaje"` | Falla (código 1) si la condición es falsa. Útil en pruebas. |
| `ASK var "¿Pregunta?"` | Lee una línea de la consola. |
| `OPEN "notepad.exe"` / `OPEN "https://..."` / `OPEN "C:\archivo.pdf"` | Abre un programa, una URL o un archivo con su aplicación. |
| `EXEC "comando"` | Ejecuta el comando en `cmd` (Windows) o `sh` y muestra la salida. |
| `EXEC "comando" -> var` | Igual, pero guarda la salida en `var`. `EXITCODE` recoge el código. |
| `ALERT msg` | Muestra un diálogo modal y espera. |
| `NOTIFY msg [, título]` | Muestra una notificación del sistema. |
| `WRITEFILE ruta, contenido`, `APPENDFILE ruta, contenido` | Escribe o añade a un archivo. |
| `DELETEFILE ruta`, `MKDIR ruta` | Borra un archivo o crea una carpeta. |
| `CLIPSET texto` | Copia el texto al portapapeles. |
| `FOCUS "Bloc de notas"` | Activa la primera ventana cuyo título o aplicación contiene el texto. |
| `WAITWINDOW "título" [, ms]` | Espera a que aparezca una ventana (10 s por defecto). |
| `SCREENSHOT "a.png"` / `SCREENSHOT "a.png" x y ancho alto` | Captura la pantalla principal o una región. |
| `ONERROR CONTINUE` / `ONERROR STOP` | Si un error detiene el script (por defecto, sí). |
| `FAILSAFE ON` / `FAILSAFE OFF` | Activa o desactiva la parada de emergencia. |

En Windows, `acm` trabaja en **píxeles físicos**. Por eso las coordenadas coinciden con las capturas aunque la pantalla esté escalada al 125 % o al 150 %.

### Web (Chrome / Edge)

El navegador se abre al usar el primer comando web y se controla por el protocolo DevTools.

**Selectores:** si empiezan por `/` o `(` se tratan como **XPath** (`//input[@id='user']`); si no, como **CSS** (`#user`, `.btn.primary`).

**Esperas:** cada comando espera a que el elemento aparezca, hasta 10 s por defecto. Se cambia con `WEBTIMEOUT ms`.

| Comando | Descripción |
|---|---|
| `WEBBROWSER CHROME\|EDGE\|BRAVE\|"ruta.exe" [HEADLESS]` | Elige el navegador. `HEADLESS` lo abre sin ventana. |
| `WEBPROFILE "carpeta"` | Usa un perfil persistente: las sesiones iniciadas se conservan. |
| `WEBOPEN url` | Navega en la pestaña actual y espera a que cargue. Con `WEBOPEN(IGNORE) url` no espera. |
| `WEBTAB` / `WEBTAB NEW` | Abre una pestaña nueva. |
| `WEBTAB url` / `WEBTAB(IGNORE) url` | Abre una pestaña y navega a la URL. |
| `WEBTAB 2` | Cambia a la pestaña 2 (se cuentan desde 0). |
| `WEBTAB CLOSE` | Cierra la pestaña actual. |
| `WEBCLICK sel` | Hace clic en el elemento. |
| `WEBINPUT sel texto` | Vacía el campo y escribe el texto. |
| `WEBGET sel var` | Guarda en `var` el texto visible o el valor del elemento. |
| `WEBWAIT sel [EXISTS\|VISIBLE\|HIDDEN\|CLICKABLE\|GONE] [ms]` | Espera a que el elemento cumpla el estado. |
| `WEBSELECT sel "opción"` | Elige una opción de una lista desplegable por su texto o su valor. |
| `WEBCHECK sel TRUE\|FALSE` | Marca o desmarca una casilla. |
| `WEBSCROLL sel` | Desplaza la página hasta el elemento. |
| `WEBKEY "Enter"` | Pulsa una tecla dentro de la página. |
| `WEBJS "código" [-> var]` | Ejecuta JavaScript. Si el código contiene `return`, se envuelve en una función. |
| `WEBSCREENSHOT "pagina.png"` | Captura la página. |
| `WEBBACK`, `WEBRELOAD` | Atrás y recargar. |
| `WEBCLOSE` | Cierra el navegador. |
| `WEBWAITCLOSE [ms]` | Espera a que el usuario cierre el navegador (sin límite o hasta `ms`). |

Firefox no está soportado. Además, el navegador se cierra cuando termina `acm`. Para dejarlo abierto mientras lo usas, termina el script con `WEBWAITCLOSE`. También puedes usar `acm -i` o la consola interactiva.

Si el usuario cierra el navegador a mitad del script, los comandos web siguientes fallan con «el navegador se cerró» en vez de abrir otro. Solo `WEBOPEN` y `WEBTAB` abren uno nuevo.

### Funciones integradas

| Grupo | Funciones |
|---|---|
| Texto | `len`, `upper`, `lower`, `trim`, `replace(s,a,b)`, `split(s,sep)`, `join(lista,sep)`, `lines`, `contains`, `startswith`, `endswith`, `indexof`, `substr(s,inicio,largo)`, `pad(s,ancho,relleno)`, `chr`, `ord` |
| Números | `num`, `int`, `str`, `isnum`, `type`, `round(x,dec)`, `floor`, `ceil`, `abs`, `min`, `max`, `sum`, `random()`, `random(lista)`, `random(a,b)`, `RANDOM("a","b",...)`, `randint(a,b)` |
| Listas | `range(n)`, `range(a,b,paso)`, `push(lista,x)`, `slice(lista,a,b)`, `reverse`, `sort`, `unique`, `json(x)`, `parsejson(s)` |
| Sistema | `exec("cmd")`, `env("VAR")`, `exists(ruta)`, `read(ruta)`, `readlines(ruta)`, `listdir(ruta)`, `now("%Y-%m-%d %H:%M:%S")`, `time()` (ms) |
| Escritorio | `mousex()`, `mousey()`, `mousepos()`, `screenwidth()`, `screenheight()`, `pixel(x,y)`, `clipboard()`, `windows()`, `activewindow()`, `windowexists("texto")` |
| Web | `webtext(sel)`, `webexists(sel)`, `webattr(sel,nombre)`, `weburl()`, `webtitle()`, `webjs(código)`, `webtabs()` |

### Extensiones (.acmx)

Las directivas de cabecera definen un nombre y argumentos que se pasan como `nombre=valor`:

```acm
@EXTENSION "Lista de IPs"
@DESCRIPTION "Genera IPs en el Bloc de notas"
@ARG base_ip "IP base" "192.168.1"
@ARG:CHECK incluir_puerto "Incluir puerto" "false"
@ARG:SELECT protocolo "Protocolo" "http|https|ftp"          // por defecto: la primera opción
@ARG:SELECT modo "Modo" "rapido" "rapido|lento"             // valor por defecto explícito
@ARG:LIST IPS "IPs" "10.0.0.1,10.0.0.2"                     // se convierte en lista

FOR ip IN IPS
    PRINT "${protocolo}://${ip}"
ENDFOR
```

```bash
acm --info lista.acmx
acm lista.acmx protocolo=https IPS=1.1.1.1,8.8.8.8
acm lista.acmx --ask        # pregunta los argumentos por consola
```

---

## Estructura del código

| Archivo | Contenido |
|---|---|
| `src/lexer.rs` | Tokens. División en sentencias por `;` y eliminación de comentarios en línea. |
| `src/expr.rs` | Expresiones: precedencia y plantillas `${}`. |
| `src/parser.rs` | Sentencias, bloques y todos los comandos. |
| `src/interp.rs` | Intérprete, funciones integradas y ejecución de comandos. |
| `src/host.rs` | Sistema operativo: enigo, arboard, xcap, ventanas, procesos, diálogos y notificaciones. |
| `src/web.rs` | Chrome/Edge por DevTools (crate `headless_chrome`). |
| `src/extension.rs` | Directivas `@ARG` de los archivos `.acmx`. |
| `src/out.rs` | Salida de texto o JSON. |
| `ide/` | IDE de escritorio (Tauri). Es un proyecto aparte que usa `acm` como proceso (ver `ide/README.md`). |
| `vendor/headless_chrome` | Copia de `headless_chrome` 1.0.22 con un solo cambio (ver `ACM_PATCH.md`): cada llamada al navegador tardaba unos 100 ms y ahora tarda unos 2 ms. |

Pruebas: `cargo test`. Para comprobar todos los scripts sin ejecutar acciones: `acm --dry-run --no-dialogs archivo.acm`.

> En Windows, compila desde PowerShell o cmd. En Git Bash, el `link` de MSYS tapa el enlazador de MSVC y la compilación falla.
