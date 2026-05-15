import type { LogLine, ProcessInfo, ProcessStatus } from "./types";
import { spawnApp, readLines, makeLogLine, appendLog } from "./spawner";

type EventMap = {
  log: (appId: string, line: LogLine) => void;
  stateChange: (appId: string, status: ProcessStatus) => void;
  exit: (appId: string, code: number | null) => void;
};

type EventHandler<T extends keyof EventMap> = EventMap[T];

export class ProcessManager {
  private apps: Map<string, ProcessInfo> = new Map();
  private handlers: Map<keyof EventMap, Set<EventHandler<keyof EventMap>>> = new Map();

  private emit<K extends keyof EventMap>(event: K, ...args: Parameters<EventMap[K]>): void {
    const handlers = this.handlers.get(event);
    if (!handlers) return;
    for (const handler of handlers) {
      (handler as (...args: unknown[]) => void)(...args);
    }
  }

  on<K extends keyof EventMap>(event: K, handler: EventHandler<K>): void {
    if (!this.handlers.has(event)) {
      this.handlers.set(event, new Set());
    }
    this.handlers.get(event)!.add(handler);
  }

  off<K extends keyof EventMap>(event: K, handler: EventHandler<K>): void {
    this.handlers.get(event)?.delete(handler);
  }

  private updateStatus(appId: string, status: ProcessStatus): void {
    const app = this.apps.get(appId);
    if (!app) return;
    app.status = status;
    this.emit("stateChange", appId, status);
  }

  private addLog(appId: string, line: LogLine): void {
    const app = this.apps.get(appId);
    if (!app) return;
    app.logs = appendLog(app.logs, line);
    this.emit("log", appId, line);
  }

  async start(appId: string, command: string, cwd?: string): Promise<void> {
    const normalizedCwd = cwd ?? process.cwd();
    this.updateStatus(appId, "starting");

    const proc = spawnApp({ command, cwd: normalizedCwd });

    const existing = this.apps.get(appId);
    this.apps.set(appId, {
      id: appId,
      pid: proc.pid,
      command,
      cwd: normalizedCwd,
      status: "running",
      logs: existing?.logs ?? [],
      kill: proc.kill,
    });
    this.emit("stateChange", appId, "running");

    // Stream stdout
    void this.streamLogs(appId, proc.stdout, "stdout");

    // Stream stderr
    void this.streamLogs(appId, proc.stderr, "stderr");

    // Watch for exit — guard against race with restarts
    const exitPid = proc.pid;
    proc.exitCode.then((code) => {
      const app = this.apps.get(appId);
      if (!app || app.pid !== exitPid) return;
      // SIGTERM exit code is 143 (128 + 15), treat as stopped not crashed
      if (code === 0 || code === 143 || code === null) {
        this.updateStatus(appId, "stopped");
      } else {
        this.updateStatus(appId, "crashed");
      }
      this.emit("exit", appId, code);
    });
  }

  private async streamLogs(
    appId: string,
    stream: ReadableStream<Uint8Array> | null,
    streamType: "stdout" | "stderr",
  ): Promise<void> {
    try {
      for await (const line of readLines(stream)) {
        this.addLog(appId, makeLogLine(streamType, line));
      }
    } catch {
      // stream closed
    }
  }

  async stop(appId: string): Promise<void> {
    const app = this.apps.get(appId);
    if (!app || app.status === "stopped" || app.status === "stopping") return;

    this.updateStatus(appId, "stopping");

    if (app.kill) {
      try {
        app.kill();
      } catch {
        // already dead
      }
    }

    await new Promise<void>((resolve) => {
      let checks = 0;
      const maxChecks = 100; // 5 seconds timeout
      const check = () => {
        checks++;
        const a = this.apps.get(appId);
        if (!a || a.status === "stopped" || a.status === "crashed" || checks >= maxChecks) {
          if (a && a.status !== "stopped" && a.status !== "crashed") {
            this.updateStatus(appId, "stopped");
          }
          resolve();
        } else {
          setTimeout(check, 50);
        }
      };
      setTimeout(check, 50);
    });
  }

  async restart(appId: string): Promise<void> {
    const app = this.apps.get(appId);
    if (!app) return;
    const { command, cwd } = app;
    await this.stop(appId);
    await this.start(appId, command, cwd);
  }

  getStatus(appId: string): ProcessStatus | null {
    return this.apps.get(appId)?.status ?? null;
  }

  getLogs(appId: string): LogLine[] {
    return this.apps.get(appId)?.logs ?? [];
  }

  getPid(appId: string): number | null {
    return this.apps.get(appId)?.pid ?? null;
  }
}
