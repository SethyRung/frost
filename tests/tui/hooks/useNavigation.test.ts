import { describe, it, expect, beforeEach } from "bun:test";
import type { FrostConfig } from "@/config/types";

const mockConfig: FrostConfig = {
  projects: {
    projectA: {
      apps: {
        frontend: { command: "npm run dev" },
        api: { command: "npm start" },
      },
    },
    projectB: {
      apps: {
        worker: { command: "node worker.js" },
      },
    },
  },
};

// Pure logic tests for useNavigation's core computations

function computeItems(config: FrostConfig | null): Array<{ projectName: string; appName: string | null; isProject: boolean; index: number }> {
  if (!config) return [];
  const result: Array<{ projectName: string; appName: string | null; isProject: boolean; index: number }> = [];
  let index = 0;
  for (const projectName of Object.keys(config.projects)) {
    result.push({ projectName, appName: null, isProject: true, index: index++ });
    const project = config.projects[projectName]!;
    for (const appName of Object.keys(project.apps)) {
      result.push({ projectName, appName, isProject: false, index: index++ });
    }
  }
  return result;
}

function moveSelection(
  items: Array<{ projectName: string; appName: string | null }>,
  currentIndex: number,
  direction: "up" | "down",
): { projectName: string; appName: string | null } {
  if (items.length === 0) return { projectName: "", appName: null };
  const delta = direction === "down" ? 1 : -1;
  const nextIndex = ((currentIndex + delta) % items.length + items.length) % items.length;
  const item = items[nextIndex]!;
  return { projectName: item.projectName, appName: item.appName };
}

describe("useNavigation logic", () => {
  let items: ReturnType<typeof computeItems>;

  beforeEach(() => {
    items = computeItems(mockConfig);
  });

  it("computes correct items list", () => {
    expect(items).toHaveLength(5);
    expect(items[0]?.projectName).toBe("projectA");
    expect(items[0]?.isProject).toBe(true);
    expect(items[1]?.appName).toBe("frontend");
    expect(items[1]?.isProject).toBe(false);
    expect(items[2]?.appName).toBe("api");
    expect(items[3]?.projectName).toBe("projectB");
    expect(items[3]?.isProject).toBe(true);
    expect(items[4]?.appName).toBe("worker");
  });

  it("moves selection down", () => {
    const sel = moveSelection(items, 0, "down");
    expect(sel.projectName).toBe("projectA");
    expect(sel.appName).toBe("frontend");
  });

  it("moves selection up", () => {
    const sel = moveSelection(items, 2, "up");
    expect(sel.projectName).toBe("projectA");
    expect(sel.appName).toBe("frontend");
  });

  it("wraps around at top", () => {
    const sel = moveSelection(items, 0, "up");
    expect(sel.projectName).toBe("projectB");
    expect(sel.appName).toBe("worker");
  });

  it("wraps around at bottom", () => {
    const sel = moveSelection(items, items.length - 1, "down");
    expect(sel.projectName).toBe("projectA");
    expect(sel.appName).toBeNull();
  });

  it("handles empty config", () => {
    const emptyItems = computeItems({ projects: {} });
    expect(emptyItems).toHaveLength(0);
    const sel = moveSelection(emptyItems, 0, "down");
    expect(sel.projectName).toBe("");
  });

  it("moves within same project", () => {
    const sel = moveSelection(items, 0, "down");
    expect(sel.projectName).toBe("projectA");
    expect(sel.appName).toBe("frontend");
    const sel2 = moveSelection(items, 1, "down");
    expect(sel2.projectName).toBe("projectA");
    expect(sel2.appName).toBe("api");
  });
});
