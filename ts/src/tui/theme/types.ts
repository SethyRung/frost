export type ThemeMode = "dark" | "light";

export type HexColor = string;

export interface RGBA {
  r: number;
  g: number;
  b: number;
  a: number;
}

export type ThemeValue =
  | string
  | { dark: string; light: string }
  | number;

export interface ThemeDefs {
  [key: string]: string;
}

export interface ThemeJson {
  $schema?: string;
  defs?: ThemeDefs;
  theme: Record<string, ThemeValue>;
}

export interface ResolvedTheme {
  primary: RGBA;
  secondary: RGBA;
  accent: RGBA;
  error: RGBA;
  warning: RGBA;
  success: RGBA;
  info: RGBA;
  text: RGBA;
  textMuted: RGBA;
  selectedListItemText: RGBA;
  background: RGBA;
  backgroundPanel: RGBA;
  backgroundElement: RGBA;
  backgroundMenu: RGBA;
  border: RGBA;
  borderActive: RGBA;
  borderSubtle: RGBA;
  diffAdded: RGBA;
  diffRemoved: RGBA;
  diffContext: RGBA;
  diffHunkHeader: RGBA;
  diffHighlightAdded: RGBA;
  diffHighlightRemoved: RGBA;
  diffAddedBg: RGBA;
  diffRemovedBg: RGBA;
  diffContextBg: RGBA;
  diffLineNumber: RGBA;
  diffAddedLineNumberBg: RGBA;
  diffRemovedLineNumberBg: RGBA;
  markdownText: RGBA;
  markdownHeading: RGBA;
  markdownLink: RGBA;
  markdownLinkText: RGBA;
  markdownCode: RGBA;
  markdownBlockQuote: RGBA;
  markdownEmph: RGBA;
  markdownStrong: RGBA;
  markdownHorizontalRule: RGBA;
  markdownListItem: RGBA;
  markdownListEnumeration: RGBA;
  markdownImage: RGBA;
  markdownImageText: RGBA;
  markdownCodeBlock: RGBA;
  syntaxComment: RGBA;
  syntaxKeyword: RGBA;
  syntaxFunction: RGBA;
  syntaxVariable: RGBA;
  syntaxString: RGBA;
  syntaxNumber: RGBA;
  syntaxType: RGBA;
  syntaxOperator: RGBA;
  syntaxPunctuation: RGBA;
  thinkingOpacity: number;
}

export interface ThemeState {
  themes: Record<string, ThemeJson>;
  mode: ThemeMode;
  lock: ThemeMode | undefined;
  active: string;
  ready: boolean;
}

export interface ThemeStore {
  getState(): ThemeState;
  subscribe(cb: () => void): () => void;
  getActive(): string;
  has(id: string): boolean;
  set(id: string): void;
  getMode(): ThemeMode;
  setMode(mode: ThemeMode): void;
  getLock(): ThemeMode | undefined;
  lock(mode: ThemeMode): void;
  unlock(): void;
  isReady(): boolean;
  getAll(): Record<string, ThemeJson>;
  getTheme(id: string): ThemeJson | undefined;
  resolveActive(): ResolvedTheme | null;
  getActiveKeys(): string[];
}

export interface TerminalColors {
  foreground: RGBA;
  background: RGBA;
  palette: RGBA[];
  defaultForeground?: RGBA;
  defaultBackground?: RGBA;
}

export interface SyntaxStyle {
  comment: string;
  keyword: string;
  function: string;
  variable: string;
  string: string;
  number: string;
  type: string;
  operator: string;
  punctuation: string;
}
