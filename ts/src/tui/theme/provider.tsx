import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { useRenderer } from "@opentui/react";
import type { ColorInput } from "@opentui/core";
import type { ThemeMode, ThemeStore } from "./types";
import { rgbaToString } from "./resolver";
import { generateSystemTheme } from "./system";
import { loadCustomThemes } from "./sources";
import { ThemeRegistry } from "./registry";
import { FrostThemeStore, type PersistedThemeState } from "./store";

export interface ThemeContextValue {
  store: ThemeStore;
  registry: ThemeRegistry;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function useThemeContext(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) {
    throw new Error("useThemeContext must be used within a ThemeProvider");
  }
  return ctx;
}

export function useThemeStore(): ThemeStore {
  return useThemeContext().store;
}

interface ThemeProviderProps {
  children: ReactNode;
  persistedState?: PersistedThemeState | null;
  onPersist?: (state: PersistedThemeState) => void;
}

export function ThemeProvider({ children, persistedState, onPersist }: ThemeProviderProps) {
  const renderer = useRenderer();

  const [registry] = useState(() => new ThemeRegistry());
  const [store] = useState(() => new FrostThemeStore(registry));
  const [, forceUpdate] = useState(0);

  useEffect(() => {
    return store.subscribe(() => forceUpdate((n) => n + 1));
  }, [store]);

  useEffect(() => {
    if (onPersist) {
      store.setPersistCallback(onPersist);
    }
  }, [store, onPersist]);

  useEffect(() => {
    if (persistedState) {
      if (persistedState.active && registry.has(persistedState.active)) {
        store.set(persistedState.active);
      }
      if (persistedState.lock && persistedState.mode) {
        store.lock(persistedState.mode);
      } else if (persistedState.mode) {
        store.setMode(persistedState.mode);
      }
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function init() {
      const rendererMode = renderer.themeMode;
      if (rendererMode && !store.getLock()) {
        store.setMode(rendererMode as ThemeMode);
      }

      const customThemes = await loadCustomThemes();
      if (!cancelled) {
        registry.merge(customThemes);
      }

      try {
        const palette = await getTerminalPalette(renderer);
        if (!cancelled && palette) {
          const systemTheme = generateSystemTheme(palette, store.getMode());
          store.setSystemTheme(systemTheme, store.getActive());
        }
      } catch {
        // palette lookup failed
      }

      if (!cancelled) {
        store.setReady(true);
      }
    }

    init();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const handler = (mode: string) => {
      if (!store.getLock()) {
        store.setMode(mode as ThemeMode);
        regenerateSystemTheme(renderer, store.getMode(), registry, store);
      }
    };

    renderer.on("theme_mode", handler);
    return () => {
      renderer.off?.("theme_mode", handler);
    };
  }, [renderer, store, registry]);

  useEffect(() => {
    const resolved = store.resolveActive();
    if (resolved && resolved.background.a > 0) {
      try {
        renderer.setBackgroundColor(rgbaToString(resolved.background) as ColorInput);
      } catch {
        // ignore
      }
    }

    const unsubscribe = store.subscribe(() => {
      const r = store.resolveActive();
      if (r && r.background.a > 0) {
        try {
          renderer.setBackgroundColor(rgbaToString(r.background) as ColorInput);
        } catch {
          // ignore
        }
      }
    });

    return unsubscribe;
  }, [renderer, store]);

  return (
    <ThemeContext.Provider value={{ store, registry }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useResolvedTheme() {
  const { store } = useThemeContext();
  const [resolved, setResolved] = useState(() => store.resolveActive());

  useEffect(() => {
    setResolved(store.resolveActive());
    return store.subscribe(() => {
      setResolved(store.resolveActive());
    });
  }, [store]);

  return resolved;
}

export function useThemeMode(): ThemeMode {
  const { store } = useThemeContext();
  return store.getMode();
}

async function getTerminalPalette(_renderer: any) {
  try {
    return null;
  } catch {
    return null;
  }
}

function regenerateSystemTheme(
  renderer: any,
  mode: ThemeMode,
  registry: ThemeRegistry,
  store: FrostThemeStore,
) {
  getTerminalPalette(renderer)
    .then((palette) => {
      if (palette) {
        const systemTheme = generateSystemTheme(palette, mode);
        store.setSystemTheme(systemTheme, store.getActive());
      }
    })
    .catch(() => {});
}
