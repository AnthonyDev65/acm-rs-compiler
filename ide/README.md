# acm IDE

Editor de escritorio para el lenguaje `acm`, hecho con Tauri 2, TypeScript y CodeMirror 6.

El IDE **no enlaza el intérprete**: lanza el ejecutable `acm` como proceso y habla con él por su línea de comandos. Puede actualizarse `acm` sin tocar el IDE, y al revés.

## Capas

| Capa | Carpeta | Responsabilidad |
|---|---|---|
| Motor | `../` (`acm`) | Intérprete. El IDE solo usa su contrato público: `--json`, `-c`, `--info`, `-e` y `-`. |
| Backend | `src-tauri/src` | `engine.rs` localiza `acm`, lo lanza y reenvía sus eventos; `files.rs` lee y escribe archivos; `launch.rs` decide si abrir el IDE o la ventana de una extensión y registra la asociación de `.acmx`. |
| Núcleo | `src/core` | Único código del frontend que llama a Tauri (`engine`, `files`, `window`, `settings`). |
| Lenguaje | `src/lang` | Resaltado, autocompletado, ayuda flotante y errores de sintaxis para CodeMirror. No depende de Tauri: recibe la función `check` desde fuera. |
| Interfaz | `src/ui` | Barra de título, editor con pestañas, explorador, panel de salida, panel `.acmx`, diálogos y el controlador `app.ts`. |

## Qué hace

- Resaltado de comandos, palabras clave, textos con `${...}`, directivas `@ARG` y comentarios.
- Autocompletado de comandos, funciones, teclas y variables y funciones del propio script (Ctrl+Espacio).
- Ayuda al pasar el ratón por un comando o una función.
- Errores de sintaxis en vivo, comprobados con `acm -c`.
- Ejecutar (F5), simular con `--dry-run` (Ctrl+F5) y detener (Shift+F5). Al detener se cierra también lo que haya abierto el script, como el navegador.
- Marca la línea que se está ejecutando y la línea del error. En la salida, «línea N» lleva a esa línea.
- Extensiones `.acmx`: *Archivo → Nueva extensión* (Ctrl+Shift+N) crea una con plantilla. El panel derecho muestra en vivo la ventana con sus `@ARG` tal como la verá el usuario; desde ahí se añaden campos (texto, casilla, opciones, lista), se editan el nombre, la descripción y el icono, y se ejecuta con los valores elegidos. Con F5 se abre esa misma ventana antes de ejecutar. Los últimos valores se recuerdan.
- **Ventana propia para los `.acmx`.** Si abres una extensión desde el Explorador de Windows (doble clic), aparece una ventana con su formulario, aunque el IDE esté cerrado. Esa ventana ejecuta la extensión con `acm`, muestra la salida, responde a `ASK` y tiene un botón *Editar* que la abre en el IDE. Mientras se ejecuta se minimiza (se puede desactivar) para no tapar la automatización. La asociación se activa en *Ajustes → Asociar .acmx* o con `acm-ide --register`; el instalador (`npm run tauri build`) también la registra.
- Barra de título propia (sin la de Windows) con los controles de ventana, el documento activo y el estado de la ejecución. Iconos de [HugeIcons](https://hugeicons.com) (paquete gratuito).
- Consola: la línea de entrada de la salida evalúa código con `acm -e` y responde a los `ASK` del script.
- Explorador de carpeta: crear, renombrar y eliminar archivos y carpetas.

Los scripts sin guardar se pasan por stdin, así que en ellos `ASK` no recibe respuesta e `IMPORT` usa rutas relativas a la carpeta de trabajo. Cada línea de la consola se ejecuta en un proceso nuevo, por lo que las variables no se conservan entre líneas.

## Desarrollo

Necesitas Node 18+ y Rust.

```bash
cd ide
npm install
npm run tauri dev      # abre el IDE con recarga en caliente
npm run tauri build    # instalador en src-tauri/target/release/bundle
```

El intérprete se busca por este orden: la ruta indicada en Ajustes, la variable `ACM_PATH`, `../target/{release,debug}` (solo en desarrollo), el `PATH` y `~/.cargo/bin`.
