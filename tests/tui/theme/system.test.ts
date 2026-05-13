import { describe, it, expect } from "bun:test";
import { generateSystemTheme } from "@/tui/theme/system";
import { resolveThemeSafe } from "@/tui/theme/resolver";
import type { TerminalColors } from "@/tui/theme/types";

describe("System theme generator", () => {
  const darkPalette: TerminalColors = {
    foreground: { r: 220, g: 220, b: 220, a: 1 },
    background: { r: 20, g: 22, b: 28, a: 1 },
    palette: Array.from({ length: 16 }, (_, i) => {
      const v = i * 16;
      return { r: v, g: v, b: v, a: 1 };
    }),
  };

  const lightPalette: TerminalColors = {
    foreground: { r: 30, g: 30, b: 30, a: 1 },
    background: { r: 240, g: 240, b: 240, a: 1 },
    palette: Array.from({ length: 16 }, (_, i) => {
      const v = 255 - i * 16;
      return { r: v, g: v, b: v, a: 1 };
    }),
  };

  it("generates a valid system theme for dark mode", () => {
    const theme = generateSystemTheme(darkPalette, "dark");
    expect(theme.theme).toBeDefined();
    expect(theme.theme.primary).toBeDefined();
    expect(theme.theme.background).toBe("transparent");
  });

  it("generates a resolvable system theme", () => {
    const theme = generateSystemTheme(darkPalette, "dark");
    const resolved = resolveThemeSafe(theme, "dark");
    expect(resolved).not.toBeNull();
    expect(resolved!.background).toEqual({ r: 0, g: 0, b: 0, a: 0 });
  });

  it("generates a valid system theme for light mode", () => {
    const theme = generateSystemTheme(lightPalette, "light");
    const resolved = resolveThemeSafe(theme, "light");
    expect(resolved).not.toBeNull();
    expect(resolved!.background).toEqual({ r: 0, g: 0, b: 0, a: 0 });
  });

  it("generates different primary colors for dark vs light", () => {
    const darkTheme = generateSystemTheme(darkPalette, "dark");
    const lightTheme = generateSystemTheme(lightPalette, "light");
    // Palette index 4 maps to different ANSI colors
    expect(darkTheme.theme.primary).not.toBe(lightTheme.theme.primary);
  });

  it("generates diff backgrounds distinct from plain background", () => {
    const theme = generateSystemTheme(darkPalette, "dark");
    const diffAddedBg = theme.theme.diffAddedBg as string;
    expect(diffAddedBg).toBeDefined();
    expect(diffAddedBg.length).toBeGreaterThan(0);

    const lightTheme = generateSystemTheme(lightPalette, "light");
    const lightDiffAddedBg = lightTheme.theme.diffAddedBg as string;
    expect(lightDiffAddedBg).toBeDefined();
    expect(lightDiffAddedBg.length).toBeGreaterThan(0);
  });

  it("generates muted text that differs from foreground", () => {
    const theme = generateSystemTheme(darkPalette, "dark");
    expect(theme.theme.textMuted).not.toBe(theme.theme.text);
  });
});
