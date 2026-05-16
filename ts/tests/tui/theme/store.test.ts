import { describe, it, expect, beforeEach } from "bun:test";
import { ThemeRegistry } from "@/tui/theme/registry";
import { FrostThemeStore } from "@/tui/theme/store";
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

describe("FrostThemeStore", () => {
  let registry: ThemeRegistry;
  let store: FrostThemeStore;

  beforeEach(() => {
    registry = new ThemeRegistry();
    store = new FrostThemeStore(registry);
  });

  it("starts with default theme active", () => {
    expect(store.getActive()).toBe(DEFAULT_THEME_ID);
    expect(store.getMode()).toBe("dark");
    expect(store.getLock()).toBeUndefined();
    expect(store.isReady()).toBe(false);
  });

  it("sets active theme", () => {
    registry.add("test", testTheme);
    store.set("test");
    expect(store.getActive()).toBe("test");
  });

  it("ignores setting non-existent theme", () => {
    store.set("nonexistent");
    expect(store.getActive()).toBe(DEFAULT_THEME_ID);
  });

  it("sets mode", () => {
    store.setMode("light");
    expect(store.getMode()).toBe("light");
  });

  it("locks mode", () => {
    store.lock("light");
    expect(store.getLock()).toBe("light");
    expect(store.getMode()).toBe("light");
  });

  it("unlocks mode", () => {
    store.lock("light");
    store.unlock();
    expect(store.getLock()).toBeUndefined();
  });

  it("resolves active theme", () => {
    registry.add("test", testTheme);
    store.set("test");
    const resolved = store.resolveActive();
    expect(resolved).not.toBeNull();
    expect(resolved!.primary.r).toBe(0xff);
    expect(resolved!.primary.g).toBe(0x00);
  });

  it("returns null for unresolvable theme", () => {
    const badTheme: ThemeJson = { theme: { primary: "missingRef" } };
    registry.add("bad", badTheme);
    store.set("bad");
    expect(store.resolveActive()).toBeNull();
  });

  it("notifies subscribers on changes", () => {
    const changes: string[] = [];
    const unsub = store.subscribe(() => changes.push("change"));

    registry.add("test", testTheme);
    store.set("test");
    expect(changes.length).toBeGreaterThanOrEqual(1);

    unsub();
  });

  it("unsubscribe stops notifications", () => {
    let count = 0;
    const unsub = store.subscribe(() => count++);
    unsub();

    store.setMode("light");
    const prev = count;
    store.setMode("dark");
    expect(count).toBe(prev);
  });

  it("persists state through callback", () => {
    const persisted: Array<{ active: string; mode?: string; lock?: string }> = [];
    store.setPersistCallback((s) => persisted.push(s));

    registry.add("test", testTheme);
    store.set("test");
    expect(persisted.length).toBeGreaterThanOrEqual(1);
    expect(persisted[persisted.length - 1]!.active).toBe("test");
  });

  it("merges themes through store", () => {
    store.merge({ merged: testTheme });
    expect(store.has("merged")).toBe(true);
  });

  it("upserts themes through store", () => {
    store.upsert("upserted", testTheme);
    expect(store.has("upserted")).toBe(true);
  });

  it("removes themes and falls back on active removal", () => {
    registry.add("secondary", testTheme);
    store.set("secondary");
    store.remove("secondary");
    expect(store.getActive()).toBe(DEFAULT_THEME_ID);
  });

  it("sets system theme", () => {
    store.setSystemTheme(testTheme);
    expect(store.has("system")).toBe(true);
  });

  it("falls back when system theme is removed and was active", () => {
    store.setSystemTheme(testTheme);
    store.set("system");
    store.setSystemTheme(null);
    expect(store.getActive()).toBe(DEFAULT_THEME_ID);
  });

  it("resolves active theme in dark mode", () => {
    const themeWithVariants: ThemeJson = {
      theme: {
        ...testTheme.theme,
        primary: { dark: "#111111", light: "#eeeeee" },
      },
    };
    registry.add("variants", themeWithVariants);
    store.set("variants");
    store.setMode("dark");
    const dark = store.resolveActive();
    expect(dark!.primary.r).toBe(0x11);

    store.setMode("light");
    const light = store.resolveActive();
    expect(light!.primary.r).toBe(0xee);
  });

  it("ready flag is settable", () => {
    expect(store.isReady()).toBe(false);
    store.setReady(true);
    expect(store.isReady()).toBe(true);
  });
});
