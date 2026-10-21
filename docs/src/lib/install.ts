// Comandos de instalación por sistema. El sistema activo lo decide <html data-os>,
// que fija el script de Base.astro (detección + elección guardada en localStorage).
const RAW = "https://raw.githubusercontent.com/AnthonyDev65/acm-rs-compiler/main";

export type OsId = "windows" | "macos" | "linux";

export const OSES: { id: OsId; label: string; prompt: string; install: string; where: string }[] = [
  {
    id: "windows",
    label: "Windows",
    prompt: ">",
    install: `irm ${RAW}/install.ps1 | iex`,
    where: "En PowerShell. Deja acm.exe en %LOCALAPPDATA%\\Programs\\acm\\bin y lo añade al PATH. No necesitas Rust.",
  },
  {
    id: "macos",
    label: "macOS",
    prompt: "$",
    install: `curl -fsSL ${RAW}/install.sh | sh`,
    where: "En la Terminal. Deja acm en ~/.local/bin. Después concede a la terminal Accesibilidad y Grabación de pantalla.",
  },
  {
    id: "linux",
    label: "Linux",
    prompt: "$",
    install: `curl -fsSL ${RAW}/install.sh | sh`,
    where: "En una sesión X11. Deja acm en ~/.local/bin; si esa carpeta no está en el PATH, el instalador te dice cómo añadirla.",
  },
];
