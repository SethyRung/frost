import { describe, it, expect, beforeEach } from "bun:test";
import { ThemeRegistry } from "@/tui/theme/registry";
import { DEFAULT_THEME_ID } from "@/tui/theme/builtin";
import type { ThemeJson } from "@/tui/theme/types";

const testTheme: ThemeJson = {
  theme: {
    primary: "#ff0000",
    secondary: "#00ff00",
    accent: "#0000ff",
    error: "#ff0000",
    warning: "#ff0000",
    success: "#00ff00",
    info: "#0000ff",
    text: "#ffffff",
    textMuted: "#cccccc",
    selectedListItemText: "#000000",
    background: "#000000",
    backgroundPanel: "#111111",
    backgroundElement: "#222222",
    backgroundMenu: "#222222",
    border: "#333333",
    borderActive: "#444444",
    borderSubtle: "#555555",
    diffAdded: "#00ff00",
    diffRemoved: "#ff0000",
    diffContext: "#ffffff",
    diffHunkHeader: "#ffffff",
    diffHighlightAdded: "#00ff00",
    diffHighlightRemoved: "#ff0000",
    diffAddedBg: "#001100",
    diffRemovedBg: "#110000",
    diffContextBg: "#000000",
    diffLineNumber: "#666666",
    diffAddedLineNumberBg: "#001100",
    diffRemovedLineNumberBg: "#110000",
    markdownText: "#ffffff",
    markdownHeading: "#ffffff",
    markdownLink: "#ffffff",
    markdownLinkText: "#ffffff",
    markdownCode: "#ffffff",
    markdownBlockQuote: "#ffffff",
    markdownEmph: "#ffffff",
    markdownStrong: "#ffffff",
    markdownHorizontalRule: "#ffffff",
    markdownListItem: "#ffffff",
    markdownListEnumeration: "#ffffff",
    markdownImage: "#ffffff",
    markdownImageText: "#ffffff",
    markdownCodeBlock: "#ffffff",
    syntaxComment: "#cccccc",
    syntaxKeyword: "#ffffff",
    syntaxFunction: "#ffffff",
    syntaxVariable: "#ffffff",
    syntaxString: "#ffffff",
    syntaxNumber: "#ffffff",
    syntaxType: "#ffffff",
    syntaxOperator: "#ffffff",
    syntaxPunctuation: "#ffffff",
    thinkingOpacity: 0.6,
  },
};

describe("ThemeRegistry", () => {
  let registry: ThemeRegistry;

  beforeEach(() => {
    registry = new ThemeRegistry();
  });

  it("starts with built-in themes", () => {
    expect(registry.has(DEFAULT_THEME_ID)).toBe(true);
  });

  it("adds a new theme", () => {
    const result = registry.add("test-theme", testTheme);
    expect(result).toBe(true);
    expect(registry.has("test-theme")).toBe(true);
  });

  it("rejects duplicate add", () => {
    registry.add("test-theme", testTheme);
    const result = registry.add("test-theme", testTheme);
    expect(result).toBe(false);
  });

  it("upsert overwrites existing theme", () => {
    const themeA: ThemeJson = { theme: { ...testTheme.theme, primary: "#111111" } };
    const themeB: ThemeJson = { theme: { ...testTheme.theme, primary: "#222222" } };
    registry.add("test", themeA);
    registry.upsert("test", themeB);
    const retrieved = registry.get("test");
    expect(retrieved!.theme.primary).toBe("#222222");
  });

  it("returns all theme ids", () => {
    registry.add("test-a", testTheme);
    registry.add("test-b", testTheme);
    const ids = registry.getIds();
    expect(ids).toContain(DEFAULT_THEME_ID);
    expect(ids).toContain("test-a");
    expect(ids).toContain("test-b");
  });

  it("removes a theme", () => {
    registry.add("test", testTheme);
    expect(registry.remove("test")).toBe(true);
    expect(registry.has("test")).toBe(false);
  });

  it("returns false when removing non-existent theme", () => {
    expect(registry.remove("nonexistent")).toBe(false);
  });

  it("merges multiple themes", () => {
    registry.merge({ "custom-a": testTheme, "custom-b": testTheme });
    expect(registry.has("custom-a")).toBe(true);
    expect(registry.has("custom-b")).toBe(true);
  });

  it("includes system theme in getAll", () => {
    registry.setSystemTheme(testTheme);
    const all = registry.getAll();
    expect(all.system).toBeDefined();
    expect(all.system!.theme.primary).toBe("#ff0000");
  });

  it("excludes system theme from getActiveIds", () => {
    registry.setSystemTheme(testTheme);
    const ids = registry.getActiveIds();
    expect(ids).not.toContain("system");
  });

  it("resets to builtins", () => {
    registry.add("custom", testTheme);
    registry.resetToBuiltins();
    expect(registry.has("custom")).toBe(false);
    expect(registry.has(DEFAULT_THEME_ID)).toBe(true);
  });

  it("clear removes all themes", () => {
    registry.clear();
    expect(registry.has(DEFAULT_THEME_ID)).toBe(false);
    expect(registry.getIds()).toHaveLength(0);
  });
});
