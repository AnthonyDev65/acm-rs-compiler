// Preferencias del IDE guardadas en el almacenamiento local de la ventana.

export interface Settings {
  /** Ruta del intérprete elegida a mano; vacía = detectar automáticamente. */
  acmPath: string;
  workspace: string | null;
  openFiles: string[];
  activeFile: string | null;
  noDialogs: boolean;
  keepGoing: boolean;
  noFailsafe: boolean;
  /** Muestra en la salida cada sentencia ejecutada. */
  showSteps: boolean;
  /** Últimos valores de los @ARG de cada extensión, por ruta. */
  argValues: Record<string, Record<string, string>>;
  fontSize: number;
  sidebarHidden: boolean;
  /** Oculta el panel con la ventana de las extensiones .acmx. */
  formHidden: boolean;
  /** Minimiza la ventana de una extensión mientras se ejecuta. */
  runnerMinimize: boolean;
}

const KEY = "acm-ide.settings";

const DEFAULTS: Settings = {
  acmPath: "",
  workspace: null,
  openFiles: [],
  activeFile: null,
  noDialogs: false,
  keepGoing: false,
  noFailsafe: false,
  showSteps: false,
  argValues: {},
  fontSize: 14,
  sidebarHidden: false,
  formHidden: false,
  runnerMinimize: true,
};

function load(): Settings {
  try {
    return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(KEY) ?? "{}") };
  } catch {
    return { ...DEFAULTS };
  }
}

export const settings: Settings = load();

export function saveSettings(patch: Partial<Settings> = {}): void {
  Object.assign(settings, patch);
  localStorage.setItem(KEY, JSON.stringify(settings));
}
