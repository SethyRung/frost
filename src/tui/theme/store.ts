import type { ResolvedTheme, ThemeJson, ThemeMode, ThemeState, ThemeStore } from "./types";
import { resolveTheme } from "./resolver";
import { DEFAULT_THEME_ID } from "./builtin";
import { ThemeRegistry } from "./registry";

type Listener = () => void;

export interface PersistedThemeState {
  active: string;
  mode?: ThemeMode;
  lock?: ThemeMode;
}

export class FrostThemeStore implements ThemeStore {
  private registry: ThemeRegistry;
  private state: ThemeState;
  private listeners: Set<Listener> = new Set();
  private resolvedCache: Map<string, ResolvedTheme> = new Map();
  private persistCallback: ((state: PersistedThemeState) => void) | null = null;

  constructor(registry: ThemeRegistry) {
    this.registry = registry;
    this.state = {
      themes: registry.getAll(),
      mode: "dark",
      lock: undefined,
      active: DEFAULT_THEME_ID,
      ready: false,
    };
  }

  setPersistCallback(cb: (state: PersistedThemeState) => void): void {
    this.persistCallback = cb;
  }

  private notify(): void {
    this.state.themes = this.registry.getAll();
    for (const cb of this.listeners) {
      try {
        cb();
      } catch {
        // ignore listener errors
      }
    }
  }

  private persist(): void {
    if (this.persistCallback) {
      this.persistCallback({
        active: this.state.active,
        mode: this.state.lock ? this.state.mode : undefined,
        lock: this.state.lock,
      });
    }
  }

  private invalidateCache(): void {
    this.resolvedCache.clear();
  }

  getState(): ThemeState {
    return { ...this.state, themes: this.registry.getAll() };
  }

  subscribe(cb: Listener): () => void {
    this.listeners.add(cb);
    return () => {
      this.listeners.delete(cb);
    };
  }

  getActive(): string {
    return this.state.active;
  }

  has(id: string): boolean {
    if (id === "system") return this.registry.getSystemTheme() !== null;
    return this.registry.has(id);
  }

  set(id: string): void {
    if (!this.registry.has(id)) return;
    if (this.state.active === id) return;
    this.state.active = id;
    this.invalidateCache();
    this.notify();
    this.persist();
  }

  getMode(): ThemeMode {
    return this.state.mode;
  }

  setMode(mode: ThemeMode): void {
    if (this.state.mode === mode) return;
    this.state.mode = mode;
    this.invalidateCache();
    this.notify();
    if (this.state.lock) {
      this.persist();
    }
  }

  getLock(): ThemeMode | undefined {
    return this.state.lock;
  }

  lock(mode: ThemeMode): void {
    this.state.lock = mode;
    this.state.mode = mode;
    this.invalidateCache();
    this.notify();
    this.persist();
  }

  unlock(): void {
    if (!this.state.lock) return;
    this.state.lock = undefined;
    this.notify();
    this.persist();
  }

  isReady(): boolean {
    return this.state.ready;
  }

  setReady(ready: boolean): void {
    this.state.ready = ready;
    this.notify();
  }

  getAll(): Record<string, ThemeJson> {
    return this.registry.getAll();
  }

  getTheme(id: string): ThemeJson | undefined {
    return this.registry.get(id);
  }

  getActiveKeys(): string[] {
    return this.registry.getIds();
  }

  resolveActive(): ResolvedTheme | null {
    const theme = this.registry.get(this.state.active);
    if (!theme) return null;

    const cacheKey = `${this.state.active}:${this.state.mode}`;
    const cached = this.resolvedCache.get(cacheKey);
    if (cached) return cached;

    try {
      const resolved = resolveTheme(theme, this.state.mode);
      this.resolvedCache.set(cacheKey, resolved);
      return resolved;
    } catch {
      return null;
    }
  }

  // Registry delegation methods

  merge(themes: Record<string, ThemeJson>): void {
    this.registry.merge(themes);
    this.invalidateCache();
    this.notify();
  }

  upsert(id: string, theme: ThemeJson): void {
    this.registry.upsert(id, theme);
    this.invalidateCache();
    this.notify();
  }

  remove(id: string): boolean {
    const result = this.registry.remove(id);
    if (result) {
      if (this.state.active === id) {
        this.state.active = this.registry.has(DEFAULT_THEME_ID)
          ? DEFAULT_THEME_ID
          : this.registry.getIds()[0] ?? DEFAULT_THEME_ID;
      }
      this.invalidateCache();
      this.notify();
    }
    return result;
  }

  setSystemTheme(theme: ThemeJson | null, currentActive?: string): void {
    const wasSystem = this.state.active === "system";
    this.registry.setSystemTheme(theme);
    this.invalidateCache();

    if (!theme && wasSystem) {
      const fallback = currentActive ?? DEFAULT_THEME_ID;
      if (this.registry.has(fallback)) {
        this.state.active = fallback;
      } else {
        this.state.active = DEFAULT_THEME_ID;
      }
    }

    this.notify();
  }

  getRegistry(): ThemeRegistry {
    return this.registry;
  }
}
