import { describe, it, expect } from "bun:test";
import {
  resolveTheme,
  parseHex,
  ResolveError,
  rgbaToString,
  TRANSPARENT,
} from "@/tui/theme/resolver";
import type { ThemeJson } from "@/tui/theme/types";
import { DEFAULT_THEMES } from "@/tui/theme/builtin";

const basicTheme: ThemeJson = {
  theme: {
    primary: "#7aa2f7",
    secondary: "#bb9af7",
    accent: "primary",
    error: "#f7768e",
    warning: "#ff9e64",
    success: "#9ece6a",
    info: "#7dcfff",
    text: "#c0caf5",
    textMuted: "#7f849c",
    selectedListItemText: "#1a1b26",
    background: "#1a1b26",
    backgroundPanel: "#24283b",
    backgroundElement: "#2a2f45",
    backgroundMenu: "#2a2f45",
    border: "#414868",
    borderActive: "#7aa2f7",
    borderSubtle: "#32364f",
    diffAdded: "#9ece6a",
    diffRemoved: "#f7768e",
    diffContext: "#c0caf5",
    diffHunkHeader: "#7aa2f7",
    diffHighlightAdded: "#9ece6a",
    diffHighlightRemoved: "#f7768e",
    diffAddedBg: "#1b3b1b",
    diffRemovedBg: "#3b1b1b",
    diffContextBg: "#24283b",
    diffLineNumber: "#565f89",
    diffAddedLineNumberBg: "#1b3b1b",
    diffRemovedLineNumberBg: "#3b1b1b",
    markdownText: "#c0caf5",
    markdownHeading: "#7aa2f7",
    markdownLink: "#73daca",
    markdownLinkText: "#73daca",
    markdownCode: "#e0af68",
    markdownBlockQuote: "#565f89",
    markdownEmph: "#bb9af7",
    markdownStrong: "#c0caf5",
    markdownHorizontalRule: "#414868",
    markdownListItem: "#c0caf5",
    markdownListEnumeration: "#bb9af7",
    markdownImage: "#73daca",
    markdownImageText: "#73daca",
    markdownCodeBlock: "#24283b",
    syntaxComment: "#565f89",
    syntaxKeyword: "#bb9af7",
    syntaxFunction: "#7aa2f7",
    syntaxVariable: "#c0caf5",
    syntaxString: "#9ece6a",
    syntaxNumber: "#ff9e64",
    syntaxType: "#e0af68",
    syntaxOperator: "#89ddff",
    syntaxPunctuation: "#565f89",
    thinkingOpacity: 0.6,
  },
};

describe("Theme Resolver", () => {
  describe("parseHex", () => {
    it("parses full hex", () => {
      const c = parseHex("#7aa2f7");
      expect(c).toEqual({ r: 0x7a, g: 0xa2, b: 0xf7, a: 1 });
    });

    it("parses hex with alpha", () => {
      const c = parseHex("#7aa2f780");
      expect(c).toEqual({ r: 0x7a, g: 0xa2, b: 0xf7, a: 0x80 / 255 });
    });

    it("parses shorthand hex", () => {
      const c = parseHex("#abc");
      expect(c).toEqual({ r: 0xaa, g: 0xbb, b: 0xcc, a: 1 });
    });

    it("throws on invalid hex", () => {
      expect(() => parseHex("not-a-color")).toThrow(ResolveError);
      expect(() => parseHex("#zzz")).toThrow(ResolveError);
    });
  });

  describe("resolveTheme", () => {
    it("resolves all tokens for default theme", () => {
      const resolved = resolveTheme(basicTheme, "dark");
      expect(resolved.primary).toEqual({ r: 0x7a, g: 0xa2, b: 0xf7, a: 1 });
      expect(resolved.accent).toEqual(resolved.primary); // reference resolved
      expect(resolved.error).toEqual({ r: 0xf7, g: 0x76, b: 0x8e, a: 1 });
      expect(resolved.thinkingOpacity).toBe(0.6);
    });

    it("resolves dark/light variants", () => {
      const theme: ThemeJson = {
        theme: {
          ...basicTheme.theme,
          primary: { dark: "#111111", light: "#eeeeee" },
        },
      };
      const dark = resolveTheme(theme, "dark");
      expect(dark.primary).toEqual({ r: 0x11, g: 0x11, b: 0x11, a: 1 });

      const light = resolveTheme(theme, "light");
      expect(light.primary).toEqual({ r: 0xee, g: 0xee, b: 0xee, a: 1 });
    });

    it("resolves through defs", () => {
      const theme: ThemeJson = {
        defs: { myBg: "#ff0000" },
        theme: {
          ...basicTheme.theme,
          background: "myBg",
        },
      };
      const resolved = resolveTheme(theme, "dark");
      expect(resolved.background).toEqual({ r: 0xff, g: 0x00, b: 0x00, a: 1 });
    });

    it("resolves transparent value", () => {
      const theme: ThemeJson = {
        theme: {
          ...basicTheme.theme,
          background: "transparent",
        },
      };
      const resolved = resolveTheme(theme, "dark");
      expect(resolved.background).toEqual(TRANSPARENT);
    });

    it("resolves none value", () => {
      const theme: ThemeJson = {
        theme: {
          ...basicTheme.theme,
          background: "none",
        },
      };
      const resolved = resolveTheme(theme, "dark");
      expect(resolved.background).toEqual(TRANSPARENT);
    });

    it("applies selectedListItemText fallback to background", () => {
      const { selectedListItemText: _, ...rest } = basicTheme.theme;
      const theme: ThemeJson = { theme: rest };
      const resolved = resolveTheme(theme, "dark");
      expect(resolved.selectedListItemText).toEqual(resolved.background);
    });

    it("applies backgroundMenu fallback to backgroundElement", () => {
      const { backgroundMenu: _, ...rest } = basicTheme.theme;
      const theme: ThemeJson = { theme: rest };
      const resolved = resolveTheme(theme, "dark");
      expect(resolved.backgroundMenu).toEqual(resolved.backgroundElement);
    });

    it("resolves default theme from builtins", () => {
      const opencode = DEFAULT_THEMES.opencode;
      expect(opencode).toBeDefined();
      const resolved = resolveTheme(opencode!, "dark");
      expect(resolved.text.r).toBeGreaterThan(0);
      expect(resolved.background.r).toBeLessThan(100);
    });
  });

  describe("circular references", () => {
    it("detects direct circular reference", () => {
      const theme: ThemeJson = {
        theme: {
          ...basicTheme.theme,
          primary: "secondary",
          secondary: "primary",
        },
      };
      expect(() => resolveTheme(theme, "dark")).toThrow(ResolveError);
    });

    it("detects self-reference", () => {
      const theme: ThemeJson = {
        theme: {
          ...basicTheme.theme,
          primary: "primary",
        },
      };
      expect(() => resolveTheme(theme, "dark")).toThrow(ResolveError);
    });
  });

  describe("missing references", () => {
    it("throws on missing def reference", () => {
      const theme: ThemeJson = {
        theme: {
          ...basicTheme.theme,
          primary: "nonexistentToken",
        },
      };
      expect(() => resolveTheme(theme, "dark")).toThrow(ResolveError);
    });
  });

  describe("resolveThemeSafe", () => {
    it("returns null instead of throwing", async () => {
      const { resolveThemeSafe } = await import("@/tui/theme/resolver");
      const theme: ThemeJson = {
        theme: {
          ...basicTheme.theme,
          primary: "missing",
        },
      };
      const result = resolveThemeSafe(theme, "dark");
      expect(result).toBeNull();
    });

    it("returns resolved theme for valid input", async () => {
      const { resolveThemeSafe } = await import("@/tui/theme/resolver");
      const result = resolveThemeSafe(basicTheme, "dark");
      expect(result).not.toBeNull();
      expect(result!.primary.r).toBe(0x7a);
    });
  });

  describe("rgbaToString", () => {
    it("formats opaque color as hex", () => {
      expect(rgbaToString({ r: 0x7a, g: 0xa2, b: 0xf7, a: 1 })).toBe("#7aa2f7");
    });

    it("formats transparent color as rgba", () => {
      const result = rgbaToString({ r: 0x7a, g: 0xa2, b: 0xf7, a: 0.5 });
      expect(result).toContain("rgba(");
      expect(result).toContain("0.5");
    });
  });

  describe("ansiToRgba", () => {
    it("maps ANSI 0-15 correctly", async () => {
      const { ansiToRgba } = await import("@/tui/theme/resolver");
      expect(ansiToRgba(0)).toEqual({ r: 0, g: 0, b: 0, a: 1 });
      expect(ansiToRgba(15)).toEqual({ r: 255, g: 255, b: 255, a: 1 });
    });

    it("maps ANSI 16-231 to 6x6x6 cube", async () => {
      const { ansiToRgba } = await import("@/tui/theme/resolver");
      const c = ansiToRgba(16);
      expect(c.r).toBe(0);
      expect(c.g).toBe(0);
      expect(c.b).toBe(0);
    });

    it("maps ANSI 232-255 to grayscale", async () => {
      const { ansiToRgba } = await import("@/tui/theme/resolver");
      const c = ansiToRgba(232);
      expect(c.r).toBe(0);
      expect(c.g).toBe(0);
      expect(c.b).toBe(0);
    });
  });
});
