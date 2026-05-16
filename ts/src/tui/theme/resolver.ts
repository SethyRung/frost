import type { RGBA, ResolvedTheme, ThemeJson, ThemeMode, ThemeValue } from "./types";

export class ResolveError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ResolveError";
  }
}

const HEX_RE = /^#([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})?$/;
const SHORT_HEX_RE = /^#([0-9a-fA-F])([0-9a-fA-F])([0-9a-fA-F])([0-9a-fA-F])?$/;

export function parseHex(hex: string): RGBA {
  const short = hex.match(SHORT_HEX_RE);
  if (short) {
    return {
      r: parseInt(short[1]! + short[1], 16),
      g: parseInt(short[2]! + short[2], 16),
      b: parseInt(short[3]! + short[3], 16),
      a: short[4] ? parseInt(short[4]! + short[4], 16) / 255 : 1,
    };
  }

  const match = hex.match(HEX_RE);
  if (!match) {
    throw new ResolveError(`Invalid hex color: ${hex}`);
  }

  return {
    r: parseInt(match[1]!, 16),
    g: parseInt(match[2]!, 16),
    b: parseInt(match[3]!, 16),
    a: match[4] ? parseInt(match[4]!, 16) / 255 : 1,
  };
}

export function rgbaEqual(a: RGBA, b: RGBA): boolean {
  return a.r === b.r && a.g === b.g && a.b === b.b && a.a === b.a;
}

export function rgbaToString(rgba: RGBA): string {
  if (rgba.a === 1) {
    return `#${rgba.r.toString(16).padStart(2, "0")}${rgba.g.toString(16).padStart(2, "0")}${rgba.b.toString(16).padStart(2, "0")}`;
  }
  return `rgba(${rgba.r},${rgba.g},${rgba.b},${rgba.a})`;
}

export const TRANSPARENT: RGBA = { r: 0, g: 0, b: 0, a: 0 };

function isVariant(value: ThemeValue): value is { dark: string; light: string } {
  return typeof value === "object" && !Array.isArray(value) && "dark" in value && "light" in value;
}

export function resolveColor(
  value: ThemeValue,
  defs: Record<string, string>,
  theme: Record<string, ThemeValue>,
  mode: ThemeMode,
  chain?: string[],
): RGBA {
  const c = chain ?? [];

  if (typeof value === "number") {
    return ansiToRgba(value);
  }

  if (isVariant(value)) {
    return resolveColor(value[mode], defs, theme, mode, c);
  }

  const str = value as string;

  if (str === "transparent" || str === "none") {
    return TRANSPARENT;
  }

  if (str.startsWith("#")) {
    return parseHex(str);
  }

  if (str.startsWith("rgba(") || str.startsWith("rgb(")) {
    return parseCssRgba(str);
  }

  // It's a reference
  if (c.includes(str)) {
    throw new ResolveError(`Circular reference detected: ${c.join(" -> ")} -> ${str}`);
  }

  // Try defs first
  if (defs[str] !== undefined) {
    const defValue = defs[str]!;
    if (defValue.startsWith("#")) {
      return parseHex(defValue);
    }
    if (defValue.startsWith("rgba(") || defValue.startsWith("rgb(")) {
      return parseCssRgba(defValue);
    }
    // Recursive def reference
    return resolveColor(defValue, defs, theme, mode, [...c, str]);
  }

  // Try theme tokens
  const themeValue = theme[str];
  if (themeValue !== undefined) {
    return resolveColor(themeValue, defs, theme, mode, [...c, str]);
  }

  throw new ResolveError(`Missing reference: ${str}`);
}

function parseCssRgba(str: string): RGBA {
  const inner = str.replace(/^rgba?\(/, "").replace(/\)$/, "");
  const parts = inner.split(",").map((p) => p.trim());
  return {
    r: parseInt(parts[0]!, 10),
    g: parseInt(parts[1]!, 10),
    b: parseInt(parts[2]!, 10),
    a: parts[3] ? parseFloat(parts[3]) : 1,
  };
}

export function ansiToRgba(index: number): RGBA {
  // Basic ANSI 16-color mapping
  const ansi16: RGBA[] = [
    { r: 0, g: 0, b: 0, a: 1 },
    { r: 128, g: 0, b: 0, a: 1 },
    { r: 0, g: 128, b: 0, a: 1 },
    { r: 128, g: 128, b: 0, a: 1 },
    { r: 0, g: 0, b: 128, a: 1 },
    { r: 128, g: 0, b: 128, a: 1 },
    { r: 0, g: 128, b: 128, a: 1 },
    { r: 192, g: 192, b: 192, a: 1 },
    { r: 128, g: 128, b: 128, a: 1 },
    { r: 255, g: 0, b: 0, a: 1 },
    { r: 0, g: 255, b: 0, a: 1 },
    { r: 255, g: 255, b: 0, a: 1 },
    { r: 0, g: 0, b: 255, a: 1 },
    { r: 255, g: 0, b: 255, a: 1 },
    { r: 0, g: 255, b: 255, a: 1 },
    { r: 255, g: 255, b: 255, a: 1 },
  ];

  if (index >= 0 && index < 16) {
    return ansi16[index]!;
  }

  if (index >= 16 && index < 232) {
    const i = index - 16;
    const r = (i / 36) % 6;
    const g = (i / 6) % 6;
    const b = i % 6;
    return {
      r: Math.round((r / 5) * 255),
      g: Math.round((g / 5) * 255),
      b: Math.round((b / 5) * 255),
      a: 1,
    };
  }

  if (index >= 232 && index < 256) {
    const gray = Math.round(((index - 232) / 23) * 255);
    return { r: gray, g: gray, b: gray, a: 1 };
  }

  return { r: 0, g: 0, b: 0, a: 1 };
}

function resolveOptional(
  key: string,
  defs: Record<string, string>,
  theme: Record<string, ThemeValue>,
  mode: ThemeMode,
): RGBA | undefined {
  const value = theme[key];
  if (value === undefined) return undefined;
  try {
    return resolveColor(value, defs, theme, mode);
  } catch {
    return undefined;
  }
}

export function resolveTheme(themeJson: ThemeJson, mode: ThemeMode): ResolvedTheme {
  const { defs = {}, theme } = themeJson;

  const r = (key: string): RGBA => {
    const value = theme[key];
    if (value === undefined) {
      throw new ResolveError(`Missing theme token: ${key}`);
    }
    return resolveColor(value, defs, theme, mode);
  };

  const resolved: ResolvedTheme = {
    primary: r("primary"),
    secondary: r("secondary"),
    accent: r("accent"),
    error: r("error"),
    warning: r("warning"),
    success: r("success"),
    info: r("info"),
    text: r("text"),
    textMuted: r("textMuted"),
    selectedListItemText: resolveOptional("selectedListItemText", defs, theme, mode) ?? r("background"),
    background: r("background"),
    backgroundPanel: r("backgroundPanel"),
    backgroundElement: r("backgroundElement"),
    backgroundMenu: resolveOptional("backgroundMenu", defs, theme, mode) ?? r("backgroundElement"),
    border: r("border"),
    borderActive: r("borderActive"),
    borderSubtle: r("borderSubtle"),
    diffAdded: r("diffAdded"),
    diffRemoved: r("diffRemoved"),
    diffContext: r("diffContext"),
    diffHunkHeader: r("diffHunkHeader"),
    diffHighlightAdded: r("diffHighlightAdded"),
    diffHighlightRemoved: r("diffHighlightRemoved"),
    diffAddedBg: r("diffAddedBg"),
    diffRemovedBg: r("diffRemovedBg"),
    diffContextBg: r("diffContextBg"),
    diffLineNumber: r("diffLineNumber"),
    diffAddedLineNumberBg: r("diffAddedLineNumberBg"),
    diffRemovedLineNumberBg: r("diffRemovedLineNumberBg"),
    markdownText: r("markdownText"),
    markdownHeading: r("markdownHeading"),
    markdownLink: r("markdownLink"),
    markdownLinkText: r("markdownLinkText"),
    markdownCode: r("markdownCode"),
    markdownBlockQuote: r("markdownBlockQuote"),
    markdownEmph: r("markdownEmph"),
    markdownStrong: r("markdownStrong"),
    markdownHorizontalRule: r("markdownHorizontalRule"),
    markdownListItem: r("markdownListItem"),
    markdownListEnumeration: r("markdownListEnumeration"),
    markdownImage: r("markdownImage"),
    markdownImageText: r("markdownImageText"),
    markdownCodeBlock: r("markdownCodeBlock"),
    syntaxComment: r("syntaxComment"),
    syntaxKeyword: r("syntaxKeyword"),
    syntaxFunction: r("syntaxFunction"),
    syntaxVariable: r("syntaxVariable"),
    syntaxString: r("syntaxString"),
    syntaxNumber: r("syntaxNumber"),
    syntaxType: r("syntaxType"),
    syntaxOperator: r("syntaxOperator"),
    syntaxPunctuation: r("syntaxPunctuation"),
    thinkingOpacity: typeof theme.thinkingOpacity === "number" ? theme.thinkingOpacity : 0.6,
  };

  return resolved;
}

export function resolveThemeSafe(themeJson: ThemeJson, mode: ThemeMode): ResolvedTheme | null {
  try {
    return resolveTheme(themeJson, mode);
  } catch {
    return null;
  }
}
