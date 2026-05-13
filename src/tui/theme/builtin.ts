import type { ThemeJson } from "./types";

export const DEFAULT_THEME_ID = "opencode";

const opencodeTheme: ThemeJson = {
  $schema: "https://frost.sh/theme.json",
  defs: {
    bg: "#1a1b26",
    fg: "#c0caf5",
    accentBlue: "#7aa2f7",
  },
  theme: {
    primary: { dark: "accentBlue", light: "#2f6feb" },
    secondary: { dark: "#bb9af7", light: "#7b5bb6" },
    accent: "primary",
    error: "#f7768e",
    warning: "#ff9e64",
    success: "#9ece6a",
    info: "#7dcfff",
    text: { dark: "fg", light: "#1f2328" },
    textMuted: { dark: "#7f849c", light: "#656d76" },
    selectedListItemText: { dark: "#1a1b26", light: "#ffffff" },
    background: { dark: "bg", light: "#ffffff" },
    backgroundPanel: { dark: "#24283b", light: "#f6f8fa" },
    backgroundElement: { dark: "#2a2f45", light: "#eef1f5" },
    backgroundMenu: { dark: "#2a2f45", light: "#eef1f5" },
    border: { dark: "#414868", light: "#d0d7de" },
    borderActive: { dark: "#7aa2f7", light: "#2f6feb" },
    borderSubtle: { dark: "#32364f", light: "#eaeef2" },
    diffAdded: "#9ece6a",
    diffRemoved: "#f7768e",
    diffContext: { dark: "#c0caf5", light: "#1f2328" },
    diffHunkHeader: { dark: "#7aa2f7", light: "#2f6feb" },
    diffHighlightAdded: "#9ece6a",
    diffHighlightRemoved: "#f7768e",
    diffAddedBg: { dark: "#1b3b1b", light: "#d9f0d9" },
    diffRemovedBg: { dark: "#3b1b1b", light: "#f0d9d9" },
    diffContextBg: { dark: "#24283b", light: "#f6f8fa" },
    diffLineNumber: { dark: "#565f89", light: "#8b949e" },
    diffAddedLineNumberBg: { dark: "#1b3b1b", light: "#d9f0d9" },
    diffRemovedLineNumberBg: { dark: "#3b1b1b", light: "#f0d9d9" },
    markdownText: { dark: "fg", light: "#1f2328" },
    markdownHeading: { dark: "#7aa2f7", light: "#2f6feb" },
    markdownLink: { dark: "#73daca", light: "#0969da" },
    markdownLinkText: { dark: "#73daca", light: "#0969da" },
    markdownCode: { dark: "#e0af68", light: "#cf222e" },
    markdownBlockQuote: { dark: "#565f89", light: "#8b949e" },
    markdownEmph: { dark: "#bb9af7", light: "#8250df" },
    markdownStrong: { dark: "#c0caf5", light: "#1f2328" },
    markdownHorizontalRule: { dark: "#414868", light: "#d0d7de" },
    markdownListItem: { dark: "fg", light: "#1f2328" },
    markdownListEnumeration: { dark: "#bb9af7", light: "#8250df" },
    markdownImage: { dark: "#73daca", light: "#0969da" },
    markdownImageText: { dark: "#73daca", light: "#0969da" },
    markdownCodeBlock: { dark: "#24283b", light: "#f6f8fa" },
    syntaxComment: { dark: "#565f89", light: "#8b949e" },
    syntaxKeyword: { dark: "#bb9af7", light: "#cf222e" },
    syntaxFunction: { dark: "#7aa2f7", light: "#8250df" },
    syntaxVariable: { dark: "#c0caf5", light: "#1f2328" },
    syntaxString: { dark: "#9ece6a", light: "#0969da" },
    syntaxNumber: { dark: "#ff9e64", light: "#0550ae" },
    syntaxType: { dark: "#e0af68", light: "#953800" },
    syntaxOperator: { dark: "#89ddff", light: "#1f2328" },
    syntaxPunctuation: { dark: "#565f89", light: "#656d76" },
    thinkingOpacity: 0.6,
  },
};

export const DEFAULT_THEMES: Record<string, ThemeJson> = {
  opencode: opencodeTheme,
};

export function getBuiltinThemeIds(): string[] {
  return Object.keys(DEFAULT_THEMES);
}
