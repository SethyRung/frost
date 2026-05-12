import { afterEach, beforeEach, describe, expect, it } from "bun:test";
import { StateStore } from "../../src/state/store";
import { mkdir, rm } from "fs/promises";

const TEST_STATE_DIR = "/tmp/frost-state-test";

describe("StateStore", () => {
  let store: StateStore;
  let originalHome: string | undefined;

  beforeEach(async () => {
    // Set a custom HOME for testing
    originalHome = process.env.HOME;
    process.env.HOME = TEST_STATE_DIR;
    // Create fresh store for each test
    store = new StateStore();
    // Clean up any existing test state
    try {
      await rm(TEST_STATE_DIR, { recursive: true, force: true });
    } catch {
      /* ignore */
    }
    // Ensure the .frost directory exists for state writes
    await mkdir(TEST_STATE_DIR, { recursive: true });
  });

  afterEach(() => {
    process.env.HOME = originalHome;
  });

  describe("load", () => {
    it("returns default state when no file exists", async () => {
      const state = await store.load();
      expect(state.version).toBe(1);
      expect(state.lastProject).toBeNull();
      expect(state.apps).toEqual({});
    });

    it("loads existing state from file", async () => {
      await Bun.write(
        `${TEST_STATE_DIR}/.frost/state.json`,
        JSON.stringify({
          version: 1,
          lastProject: "my-project",
          apps: { "my-project/frontend": { status: "running", pid: 12345 } },
        }),
      );
      const freshStore = new StateStore();
      const state = await freshStore.load();
      expect(state.lastProject).toBe("my-project");
      expect(state.apps["my-project/frontend"]?.status).toBe("running");
      expect(state.apps["my-project/frontend"]?.pid).toBe(12345);
    });
  });

  describe("setLastProject", () => {
    it("saves last project to state", async () => {
      await store.load();
      await store.setLastProject("my-project");
      expect(store.getLastProject()).toBe("my-project");
    });
  });

  describe("updateApp", () => {
    it("creates new app state if not exists", async () => {
      await store.load();
      await store.updateApp("my-project/frontend", { status: "running", pid: 12345 });
      const app = store.getAppState("my-project/frontend");
      expect(app).toEqual({ status: "running", pid: 12345 });
    });

    it("merges updates into existing app state", async () => {
      await store.load();
      await store.updateApp("my-project/frontend", { status: "running", pid: 12345 });
      await store.updateApp("my-project/frontend", { status: "stopped", pid: undefined });
      const app = store.getAppState("my-project/frontend");
      expect(app).toEqual({ status: "stopped" });
    });

    it("returns null for unknown app", () => {
      expect(store.getAppState("nonexistent")).toBeNull();
    });
  });

  describe("persistence", () => {
    it("persists state to disk", async () => {
      await store.load();
      await store.updateApp("my-project/frontend", { status: "running", pid: 999 });
      await store.setLastProject("my-project");

      const freshStore = new StateStore();
      const loaded = await freshStore.load();
      expect(loaded.lastProject).toBe("my-project");
      expect(loaded.apps["my-project/frontend"]).toEqual({ status: "running", pid: 999 });
    });
  });
});
