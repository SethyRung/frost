import { mkdir } from "fs/promises";
import type { AppState, FrostState } from "./types";
import { CURRENT_VERSION, STATE_FILE } from "./types";

function getStateDir(): string {
  const home = process.env.HOME ?? "/root";
  return `${home}/${STATE_FILE.replace(/\/[^/]*$/, "")}`;
}

function getStatePath(): string {
  const home = process.env.HOME ?? "/root";
  return `${home}/${STATE_FILE}`;
}

export class StateStore {
  private state: FrostState;
  private saveTimer: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    this.state = { version: CURRENT_VERSION, lastProject: null, apps: {} };
  }

  async load(): Promise<FrostState> {
    const path = getStatePath();
    try {
      const text = await Bun.file(path).text();
      const parsed = JSON.parse(text) as FrostState;
      this.state = parsed;
    } catch {
      this.state = { version: CURRENT_VERSION, lastProject: null, apps: {} };
    }
    return this.state;
  }

  private async saveDebounced(): Promise<void> {
    if (this.saveTimer) clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => void this.save(), 500);
  }

  async save(): Promise<void> {
    const path = getStatePath();
    const dir = getStateDir();

    // Ensure directory exists
    try {
      await mkdir(dir, { recursive: true });
    } catch {
      // might already exist
    }

    const json = JSON.stringify(this.state, null, 2);
    await Bun.write(path, json);
  }

  async setLastProject(projectId: string): Promise<void> {
    this.state.lastProject = projectId;
    await this.save();
  }

  async updateApp(appId: string, update: Partial<AppState>): Promise<void> {
    if (!this.state.apps[appId]) {
      this.state.apps[appId] = { status: "stopped" };
    }
    this.state.apps[appId] = { ...this.state.apps[appId]!, ...update };
    await this.save();
  }

  getLastProject(): string | null {
    return this.state.lastProject;
  }

  getAppState(appId: string): AppState | null {
    return this.state.apps[appId] ?? null;
  }
}
