// Cliente del intérprete acm. Es el único módulo que conoce los comandos del backend
// relacionados con acm; la interfaz trabaja solo con estos tipos.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface EngineInfo {
  path: string;
  version: string;
}

export interface CheckResult {
  ok: boolean;
  line: number | null;
  message: string | null;
  statements: number | null;
}

export type ArgType = "text" | "check" | "list" | "select";

export interface ExtensionArg {
  name: string;
  label: string;
  type: ArgType;
  default: string;
  options: string[];
}

export interface ExtensionInfo {
  name: string | null;
  description: string | null;
  author: string | null;
  icon: string | null;
  args: ExtensionArg[];
}

export interface RunRequest {
  path?: string;
  source?: string;
  eval?: string;
  cwd?: string;
  args?: string[];
  dryRun?: boolean;
  trace?: boolean;
  keepGoing?: boolean;
  noDialogs?: boolean;
  noFailsafe?: boolean;
}

/** Eventos de `acm --json` (ver README del intérprete). */
export type AcmEvent =
  | { event: "start"; script: string; statements: number }
  | { event: "print"; text: string }
  | { event: "step"; line: number; code: string }
  | { event: "dry"; line: number; action: string }
  | { event: "warning"; message: string }
  | { event: "error"; line: number | null; message: string }
  | { event: "ask"; var: string; prompt: string }
  | { event: "screenshot"; path: string }
  | { event: "done"; ok: boolean; code: number; ms?: number }
  | { event: string; [k: string]: unknown };

export type RunMessage =
  | { runId: number; kind: "event"; event: AcmEvent }
  | { runId: number; kind: "stdout" | "stderr"; text: string }
  | { runId: number; kind: "exit"; code: number | null };

export const engine = {
  locate: (custom?: string) => invoke<EngineInfo>("engine_locate", { custom: custom ?? null }),
  check: (source: string) => invoke<CheckResult>("engine_check", { source }),
  info: (source: string) => invoke<ExtensionInfo>("engine_info", { source }),
  run: (req: RunRequest) => invoke<number>("engine_run", { req }),
  stop: (runId: number) => invoke<void>("engine_stop", { runId }),
  input: (runId: number, text: string) => invoke<void>("engine_input", { runId, text }),
  onMessage: (fn: (m: RunMessage) => void): Promise<UnlistenFn> => listen<RunMessage>("acm://run", (e) => fn(e.payload)),
};
