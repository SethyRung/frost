import { describe, it, expect } from "bun:test";
import { ProcessManager } from "@/process/manager";
import type { FrostConfig } from "@/config/types";

const mockConfig: FrostConfig = {
  projects: {
    test: {
      apps: {
        sleeper: { command: "sleep 30" },
      },
    },
  },
};

function createUseProcessManagerLike(pm: ProcessManager, config: FrostConfig) {
  // Mimic what useProcessManager does: subscribe to events and expose getStatus
  const states: Record<string, { status: string; logs: unknown[] }> = {};

  pm.on("stateChange", (appId, status) => {
    const existing = states[appId] ?? { status: "stopped", logs: [] };
    states[appId] = { ...existing, status };
  });

  pm.on("log", (appId) => {
    const existing = states[appId] ?? { status: "stopped", logs: [] };
    states[appId] = { ...existing };
  });

  return {
    getStatus: (projectName: string, appName: string) => {
      const id = `${projectName}/${appName}`;
      return (states[id]?.status as "stopped" | "running" | "starting" | "stopping" | "crashed") ?? "stopped";
    },
    async toggleApp(projectName: string, appName: string) {
      const status = this.getStatus(projectName, appName);
      const id = `${projectName}/${appName}`;
      const isActive = status === "running" || status === "starting" || status === "stopping";
      if (isActive) {
        await pm.stop(id);
      } else {
        const app = config.projects[projectName]?.apps[appName];
        if (app) await pm.start(id, app.command);
      }
    },
    async toggleAll(projectName: string) {
      const project = config.projects[projectName];
      if (!project) return;
      const appNames = Object.keys(project.apps);
      const anyActive = appNames.some((appName) => {
        const status = this.getStatus(projectName, appName);
        return status === "running" || status === "starting" || status === "stopping";
      });
      if (anyActive) {
        for (const appName of appNames) {
          await pm.stop(`${projectName}/${appName}`);
        }
      } else {
        for (const appName of appNames) {
          const app = project.apps[appName];
          if (app) await pm.start(`${projectName}/${appName}`, app.command);
        }
      }
    },
  };
}

describe("toggle integration", () => {
  it("starts, stops, and restarts an app via toggle", async () => {
    const pm = new ProcessManager();
    const proc = createUseProcessManagerLike(pm, mockConfig);

    expect(proc.getStatus("test", "sleeper")).toBe("stopped");

    await proc.toggleApp("test", "sleeper");
    expect(proc.getStatus("test", "sleeper")).toBe("running");

    await new Promise((r) => setTimeout(r, 200));

    await proc.toggleApp("test", "sleeper");
    expect(proc.getStatus("test", "sleeper")).toBe("stopped");

    await proc.toggleApp("test", "sleeper");
    expect(proc.getStatus("test", "sleeper")).toBe("running");

    await proc.toggleApp("test", "sleeper");
    expect(proc.getStatus("test", "sleeper")).toBe("stopped");
  });

  it("toggleAll starts all then stops all", async () => {
    const pm = new ProcessManager();
    const proc = createUseProcessManagerLike(pm, mockConfig);

    expect(proc.getStatus("test", "sleeper")).toBe("stopped");

    await proc.toggleAll("test");
    await new Promise((r) => setTimeout(r, 200));
    expect(proc.getStatus("test", "sleeper")).toBe("running");

    await proc.toggleAll("test");
    expect(proc.getStatus("test", "sleeper")).toBe("stopped");
  });
});
