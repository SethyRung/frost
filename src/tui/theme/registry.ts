import type { ThemeJson } from "./types";
import { DEFAULT_THEMES } from "./builtin";

export class ThemeRegistry {
  private themes: Record<string, ThemeJson> = {};
  private systemTheme: ThemeJson | null = null;

  constructor() {
    // Load builtins
    this.merge(DEFAULT_THEMES);
  }

  merge(themes: Record<string, ThemeJson>): void {
    for (const [id, theme] of Object.entries(themes)) {
      this.themes[id] = theme;
    }
  }

  add(id: string, theme: ThemeJson): boolean {
    if (this.themes[id]) {
      return false;
    }
    this.themes[id] = theme;
    return true;
  }

  upsert(id: string, theme: ThemeJson): void {
    this.themes[id] = theme;
  }

  get(id: string): ThemeJson | undefined {
    return this.themes[id];
  }

  has(id: string): boolean {
    return id in this.themes;
  }

  remove(id: string): boolean {
    if (!this.themes[id]) {
      return false;
    }
    delete this.themes[id];
    return true;
  }

  getAll(): Record<string, ThemeJson> {
    const result = { ...this.themes };
    if (this.systemTheme) {
      result.system = this.systemTheme;
    }
    return result;
  }

  getIds(): string[] {
    return Object.keys(this.getAll());
  }

  getActiveIds(): string[] {
    return Object.keys(this.themes);
  }

  setSystemTheme(theme: ThemeJson | null): void {
    this.systemTheme = theme;
  }

  getSystemTheme(): ThemeJson | null {
    return this.systemTheme;
  }

  clear(): void {
    this.themes = {};
    this.systemTheme = null;
  }

  resetToBuiltins(): void {
    this.themes = {};
    this.systemTheme = null;
    this.merge(DEFAULT_THEMES);
  }
}
