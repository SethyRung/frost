import { useState, useCallback, useMemo } from "react";
import type { FrostConfig } from "@/config/types";

export type FocusArea = "sidebar" | "logs";

export interface SidebarSelection {
  projectName: string;
  appName: string | null; // null means the project header is selected
}

export function useNavigation(config: FrostConfig | null) {
  const [focus, setFocus] = useState<FocusArea>("sidebar");
  const [selection, setSelection] = useState<SidebarSelection>({ projectName: "", appName: null });

  const items = useMemo(() => {
    if (!config) return [] as Array<{ projectName: string; appName: string | null; isProject: boolean; index: number }>;
    const result: Array<{ projectName: string; appName: string | null; isProject: boolean; index: number }> = [];
    let index = 0;
    for (const projectName of Object.keys(config.projects)) {
      result.push({ projectName, appName: null, isProject: true, index: index++ });
      const project = config.projects[projectName]!;
      for (const appName of Object.keys(project.apps)) {
        result.push({ projectName, appName, isProject: false, index: index++ });
      }
    }
    return result;
  }, [config]);

  const selectedIndex = useMemo(() => {
    return items.findIndex(
      (item) =>
        item.projectName === selection.projectName &&
        item.appName === selection.appName,
    );
  }, [items, selection]);

  const moveUp = useCallback(() => {
    if (items.length === 0) return;
    const prevIndex = selectedIndex <= 0 ? items.length - 1 : selectedIndex - 1;
    const prev = items[prevIndex]!;
    setSelection({ projectName: prev.projectName, appName: prev.appName });
  }, [items, selectedIndex]);

  const moveDown = useCallback(() => {
    if (items.length === 0) return;
    const nextIndex = selectedIndex >= items.length - 1 ? 0 : selectedIndex + 1;
    const next = items[nextIndex]!;
    setSelection({ projectName: next.projectName, appName: next.appName });
  }, [items, selectedIndex]);

  const selectFirst = useCallback(() => {
    if (items.length > 0) {
      const first = items[0]!;
      setSelection({ projectName: first.projectName, appName: first.appName });
    }
  }, [items]);

  const focusSidebar = useCallback(() => setFocus("sidebar"), []);
  const focusLogs = useCallback(() => setFocus("logs"), []);
  const toggleFocus = useCallback(() => {
    setFocus((prev) => (prev === "sidebar" ? "logs" : "sidebar"));
  }, []);

  return {
    focus,
    selection,
    setSelection,
    selectedIndex,
    items,
    moveUp,
    moveDown,
    selectFirst,
    focusSidebar,
    focusLogs,
    toggleFocus,
    setFocus,
  };
}
