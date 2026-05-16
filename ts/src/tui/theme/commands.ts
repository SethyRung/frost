import type { FrostThemeStore } from "./store";
import type { ThemeMode } from "./types";

export interface ThemeCommands {
  "theme.switch": {
    handler: () => void;
  };
  "theme.switch_mode": {
    handler: () => void;
  };
  "theme.mode.lock": {
    handler: () => void;
  };
}

export function createThemeCommands(
  store: FrostThemeStore,
  openThemeDialog: () => void,
): ThemeCommands {
  return {
    "theme.switch": {
      handler: openThemeDialog,
    },
    "theme.switch_mode": {
      handler: () => {
        const current = store.getMode();
        const next: ThemeMode = current === "dark" ? "light" : "dark";
        store.lock(next);
      },
    },
    "theme.mode.lock": {
      handler: () => {
        if (store.getLock()) {
          store.unlock();
        } else {
          store.lock(store.getMode());
        }
      },
    },
  };
}

export function getThemeKeybindings() {
  return [
    { key: "<leader>t", cmd: "theme.switch", desc: "Switch theme" },
    { key: undefined, cmd: "theme.switch_mode", desc: "Toggle dark/light mode" },
    { key: undefined, cmd: "theme.mode.lock", desc: "Lock/unlock mode" },
  ];
}
