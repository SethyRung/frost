import { describe, it, expect, beforeEach } from "bun:test";
import { ProcessManager } from "../../src/process/manager";

describe("ProcessManager", () => {
  let manager: ProcessManager;

  beforeEach(() => {
    manager = new ProcessManager();
  });

  describe("start / stop", () => {
    it("starts a process and captures its output", async () => {
      const lines: string[] = [];
      manager.on("log", (_id, line) => lines.push(line.text));

      await manager.start("echo-test", "echo hello world");
      await new Promise((r) => setTimeout(r, 200));

      expect(lines.some((l) => l.includes("hello world"))).toBe(true);
    });

    it("stops a long-running process", async () => {
      await manager.start("sleep-test", "sleep 10");
      await new Promise((r) => setTimeout(r, 50));
      expect(manager.getStatus("sleep-test")).toBe("running");

      await manager.stop("sleep-test");
      expect(manager.getStatus("sleep-test")).toBe("stopped");
    });

    it("returns null status for unknown app", () => {
      expect(manager.getStatus("nonexistent")).toBeNull();
    });

    it("returns empty logs for unknown app", () => {
      expect(manager.getLogs("nonexistent")).toEqual([]);
    });
  });

  describe("logs", () => {
    it("captures stdout lines", async () => {
      const lines: string[] = [];
      manager.on("log", (_id, line) => lines.push(line.text));

      await manager.start("echo-test", "echo line1 && echo line2");
      await new Promise((r) => setTimeout(r, 200));

      expect(lines.some((l) => l.includes("line1"))).toBe(true);
      expect(lines.some((l) => l.includes("line2"))).toBe(true);
    });

    it("logs have stream and timestamp", async () => {
      const logs: { stream: string; timestamp: number }[] = [];
      manager.on("log", (_id, line) =>
        logs.push({ stream: line.stream, timestamp: line.timestamp }),
      );

      await manager.start("echo-test", "echo hello");
      await new Promise((r) => setTimeout(r, 200));

      expect(logs.length).toBeGreaterThan(0);
      expect(logs[0]!.stream).toBe("stdout");
      expect(logs[0]!.timestamp).toBeGreaterThan(0);
    });

    it("emits exit event with code", async () => {
      const exits: [string, number | null][] = [];
      manager.on("exit", (id, code) => exits.push([id, code]));

      await manager.start("echo-test", "echo hello");
      await new Promise((r) => setTimeout(r, 200));

      expect(exits).toContainEqual(["echo-test", 0]);
    });

    it("handles multiple apps independently", async () => {
      const logs1: string[] = [];
      const logs2: string[] = [];
      manager.on("log", (id, line) => {
        if (id === "app1") logs1.push(line.text);
        if (id === "app2") logs2.push(line.text);
      });

      await manager.start("app1", "echo app1-output");
      await manager.start("app2", "echo app2-output");
      await new Promise((r) => setTimeout(r, 200));

      expect(logs1.some((l) => l.includes("app1-output"))).toBe(true);
      expect(logs2.some((l) => l.includes("app2-output"))).toBe(true);
    });
  });

  describe("restart", () => {
    it("restarts a running process with a new pid", async () => {
      await manager.start("echo-test", "sleep 5");
      await new Promise((r) => setTimeout(r, 50));
      const pidBefore = manager.getPid("echo-test");
      expect(pidBefore).toBeGreaterThan(0);

      await manager.restart("echo-test");
      await new Promise((r) => setTimeout(r, 50));

      const pidAfter = manager.getPid("echo-test");
      expect(pidAfter).toBeGreaterThan(0);
      expect(pidAfter).not.toBe(pidBefore);
    });

    it("does nothing for unknown app", async () => {
      await expect(manager.restart("nonexistent")).resolves.toBeUndefined();
    });
  });
});
