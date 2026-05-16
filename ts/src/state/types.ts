export type AppStatus = "stopped" | "starting" | "running" | "stopping" | "crashed";

export interface AppState {
  status: AppStatus;
  pid?: number;
}

export interface FrostState {
  version: number;
  lastProject: string | null;
  apps: Record<string, AppState>;
}

export const CURRENT_VERSION = 1;
export const STATE_FILE = ".frost/state.json";
