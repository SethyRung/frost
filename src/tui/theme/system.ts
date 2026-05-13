import type { RGBA, TerminalColors, ThemeJson, ThemeMode } from "./types";
import { ansiToRgba, rgbaToString } from "./resolver";

function luminance(rgba: RGBA): number {
  const [r, g, b] = [rgba.r / 255, rgba.g / 255, rgba.b / 255];
  const linearize = (c: number) =>
    c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  return 0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b);
}

function blendOver(a: RGBA, b: RGBA): RGBA {
  const alpha = a.a + b.a * (1 - a.a);
  if (alpha === 0) return TRANSPARENT;
  return {
    r: Math.round((a.r * a.a + b.r * b.a * (1 - a.a)) / alpha),
    g: Math.round((a.g * a.a + b.g * b.a * (1 - a.a)) / alpha),
    b: Math.round((a.b * a.a + b.b * b.a * (1 - a.a)) / alpha),
    a: alpha,
  };
}

const TRANSPARENT: RGBA = { r: 0, g: 0, b: 0, a: 0 };

function grayscaleRamp(bgLuminance: number, steps: number): string[] {
  const ramp: string[] = [];
  const isDark = bgLuminance <= 0.5;

  for (let i = 1; i <= steps; i++) {
    const t = i / (steps + 1);
    let gray: number;
    if (isDark) {
      gray = Math.round(10 + t * 200);
    } else {
      gray = Math.round(245 - t * 200);
    }
    gray = Math.max(0, Math.min(255, gray));
    const hex = `#${gray.toString(16).padStart(2, "0")}${gray.toString(16).padStart(2, "0")}${gray.toString(16).padStart(2, "0")}`;
    ramp.push(hex);
  }
  return ramp;
}

function rampIndex(steps: number, idx: number): number {
  return Math.max(0, Math.min(steps - 1, idx));
}

export function generateSystemTheme(palette: TerminalColors, mode: ThemeMode): ThemeJson {
  const bg = palette.background;
  const fg = palette.foreground;
  const bgLum = luminance(bg);
  const isDark = bgLum <= 0.5 || mode === "dark";

  const ramp = grayscaleRamp(bgLum, 12);

  const defs: Record<string, string> = {};

  const semanticFromAnsi = (idx: number): string => {
    if (idx >= 0 && idx < palette.palette.length) {
      return rgbaToString(palette.palette[idx]!);
    }
    return rgbaToString(ansiToRgba(idx));
  };

  const rampStr = (idx: number): string => ramp[rampIndex(12, idx)]!;

  const mutedTextLum = isDark ? bgLum + 0.35 : bgLum - 0.35;
  const mutedTextGray = Math.round(Math.min(255, Math.max(0, mutedTextLum * 255)));

  const diffAlpha = isDark ? 0.22 : 0.14;
  const diffAddedBg = blendOver(
    { r: 0, g: 255, b: 0, a: diffAlpha },
    bg,
  );
  const diffRemovedBg = blendOver(
    { r: 255, g: 0, b: 0, a: diffAlpha },
    bg,
  );

  return {
    defs,
    theme: {
      primary: semanticFromAnsi(4),
      secondary: semanticFromAnsi(5),
      accent: semanticFromAnsi(6),
      error: semanticFromAnsi(1),
      warning: semanticFromAnsi(3),
      success: semanticFromAnsi(2),
      info: semanticFromAnsi(6),
      text: rgbaToString(fg),
      textMuted: `#${mutedTextGray.toString(16).padStart(2, "0")}${mutedTextGray.toString(16).padStart(2, "0")}${mutedTextGray.toString(16).padStart(2, "0")}`,
      selectedListItemText: isDark ? rampStr(0) : rampStr(11),
      background: "transparent",
      backgroundPanel: rampStr(isDark ? 1 : 10),
      backgroundElement: rampStr(isDark ? 2 : 9),
      backgroundMenu: rampStr(isDark ? 2 : 9),
      border: rampStr(isDark ? 4 : 7),
      borderActive: semanticFromAnsi(4),
      borderSubtle: rampStr(isDark ? 3 : 8),
      diffAdded: semanticFromAnsi(2),
      diffRemoved: semanticFromAnsi(1),
      diffContext: rgbaToString(fg),
      diffHunkHeader: semanticFromAnsi(4),
      diffHighlightAdded: semanticFromAnsi(2),
      diffHighlightRemoved: semanticFromAnsi(1),
      diffAddedBg: rgbaToString(diffAddedBg),
      diffRemovedBg: rgbaToString(diffRemovedBg),
      diffContextBg: rgbaToString(bg),
      diffLineNumber: rampStr(isDark ? 5 : 6),
      diffAddedLineNumberBg: rgbaToString(diffAddedBg),
      diffRemovedLineNumberBg: rgbaToString(diffRemovedBg),
      markdownText: rgbaToString(fg),
      markdownHeading: semanticFromAnsi(4),
      markdownLink: semanticFromAnsi(6),
      markdownLinkText: semanticFromAnsi(6),
      markdownCode: semanticFromAnsi(3),
      markdownBlockQuote: rampStr(isDark ? 5 : 6),
      markdownEmph: semanticFromAnsi(5),
      markdownStrong: rgbaToString(fg),
      markdownHorizontalRule: rampStr(isDark ? 4 : 7),
      markdownListItem: rgbaToString(fg),
      markdownListEnumeration: semanticFromAnsi(5),
      markdownImage: semanticFromAnsi(6),
      markdownImageText: semanticFromAnsi(6),
      markdownCodeBlock: rgbaToString(bg),
      syntaxComment: rampStr(isDark ? 5 : 6),
      syntaxKeyword: semanticFromAnsi(5),
      syntaxFunction: semanticFromAnsi(4),
      syntaxVariable: rgbaToString(fg),
      syntaxString: semanticFromAnsi(2),
      syntaxNumber: semanticFromAnsi(3),
      syntaxType: semanticFromAnsi(3),
      syntaxOperator: semanticFromAnsi(6),
      syntaxPunctuation: rampStr(isDark ? 5 : 6),
      thinkingOpacity: 0.6,
    },
  };
}
