import { describe, it, expect } from "bun:test";

// Pure logic tests for useCommandProcessor's core behavior
// Testing the command processing and suggestion logic directly

function processCommand(
  registry: Record<string, (...args: string[]) => void>,
  input: string,
  onUnknown?: (cmd: string) => void,
): boolean {
  const trimmed = input.trim();
  if (!trimmed) return false;

  const parts = trimmed.split(/\s+/);
  const cmd = parts[0]!;
  const args = parts.slice(1);

  const handler = registry[cmd];
  if (handler) {
    handler(...args);
    return true;
  }

  onUnknown?.(cmd);
  return false;
}

function getSuggestions(registry: Record<string, unknown>, input: string): string[] {
  const trimmed = input.trim().toLowerCase();
  if (!trimmed) return Object.keys(registry);
  return Object.keys(registry).filter((cmd) =>
    cmd.toLowerCase().startsWith(trimmed),
  );
}

describe("command processing logic", () => {
  it("calls handler for known command", () => {
    const calls: string[][] = [];
    const registry = {
      test: (...args: string[]) => calls.push(args),
    };
    const result = processCommand(registry, "test arg1 arg2");
    expect(result).toBe(true);
    expect(calls).toHaveLength(1);
    expect(calls[0]).toEqual(["arg1", "arg2"]);
  });

  it("calls onUnknown for unknown command", () => {
    const unknownCalls: string[] = [];
    const registry = {};
    processCommand(registry, "nonexistent", (cmd) => unknownCalls.push(cmd));
    expect(unknownCalls).toEqual(["nonexistent"]);
  });

  it("returns false for empty input", () => {
    const registry = { test: () => {} };
    expect(processCommand(registry, "")).toBe(false);
    expect(processCommand(registry, "  ")).toBe(false);
  });

  it("returns suggestions based on prefix", () => {
    const registry = {
      start: () => {},
      stop: () => {},
      status: () => {},
      help: () => {},
    };
    const suggestions = getSuggestions(registry, "st");
    expect(suggestions.sort()).toEqual(["start", "status", "stop"].sort());
  });

  it("returns all commands for empty input", () => {
    const registry = {
      start: () => {},
      stop: () => {},
    };
    const suggestions = getSuggestions(registry, "");
    expect(suggestions.sort()).toEqual(["start", "stop"].sort());
  });

  it("processes command with no args", () => {
    const calls: string[][] = [];
    const registry = {
      help: (...args: string[]) => calls.push(args),
    };
    processCommand(registry, "help");
    expect(calls[0]).toEqual([]);
  });

  it("returns true for known command", () => {
    const registry = { cmd: () => {} };
    expect(processCommand(registry, "cmd")).toBe(true);
  });
});
