import { describe, it, expect } from "bun:test";
import { ProcessManager } from "@/process/manager";
import type { FrostConfig } from "@/config/types";

const mockConfig: FrostConfig = {
  projects: {
    testProject: {
      apps: {
        frontend: { command: "npm run dev" },
        api: { command: "npm start" },
      },
    },
  },
};

function flatApps(config: FrostConfig | null) {
  if (!config) return [];
  const items: Array<{ projectName: string; appName: string; command: string }> = [];
  for (const [projectName, project] of Object.entries(config.projects)) {
    for (const [appName, app] of Object.entries(project.apps)) {
      items.push({ projectName, appName, command: app.command });
    }
  }
  return items;
}

describe("ProcessManager logic", () => {
  it("initializes with no apps", () => {
    const pm = new ProcessManager();
    // No apps started yet
    expect(pm.getStatus("testProject/frontend")).toBeNull();
  });

  it("flatApps returns all apps from config", () => {
    const apps = flatApps(mockConfig);
    expect(apps).toHaveLength(2);
    expect(apps[0]?.appName).toBe("frontend");
    expect(apps[1]?.appName).toBe("api");
  });

  it("flatApps returns empty for null config", () => {
    expect(flatApps(null)).toEqual([]);
  });

  it("flatApps returns empty for empty config", () => {
    expect(flatApps({ projects: {} })).toEqual([]);
  });

  it("ProcessManager tracks process states", async () => {
    const pm = new ProcessManager();
    // Start a process
    await pm.start("testProject/frontend", "echo hello", ".");
    expect(pm.getStatus("testProject/frontend")).toBe("running");

    // Wait briefly for process to complete
    await new Promise((r) => setTimeout(r, 100));
    expect(pm.getStatus("testProject/frontend")).toBe("stopped");
  });

  it("ProcessManager getStatus returns null for unknown app", () => {
    const pm = new ProcessManager();
    expect(pm.getStatus("nonexistent")).toBeNull();
  });

  it("ProcessManager getLogs returns array", () => {
    const pm = new ProcessManager();
    expect(pm.getLogs("test")).toEqual([]);
  });
});
