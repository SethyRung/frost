import { useState, useMemo } from "react";
import { useKeyboard } from "@opentui/react";
import type { FrostConfig } from "@/config/types";
import type { ProcessManager } from "@/process/manager";
import { useThemeStore, useResolvedTheme } from "@/tui/theme";

import { useProcessManager } from "@/tui/hooks/useProcessManager";
import { useNavigation } from "@/tui/hooks/useNavigation";

import { Sidebar } from "@/tui/components/Sidebar";
import { LogViewer } from "@/tui/components/LogViewer";
import { CommandBar } from "@/tui/components/CommandBar";
import { CommandPalette } from "@/tui/components/CommandPalette";
import { SearchDialog } from "@/tui/components/SearchDialog";
import { ThemeDialog } from "@/tui/components/ThemeDialog";

interface DashboardProps {
  config: FrostConfig;
  processManager: ProcessManager;
}

type Overlay = "none" | "palette" | "search" | "themes";

export function Dashboard({ config, processManager }: DashboardProps) {
  const themeStore = useThemeStore();
  const resolvedTheme = useResolvedTheme();
  const nav = useNavigation(config);
  const proc = useProcessManager(processManager, config);
  const [overlay, setOverlay] = useState<Overlay>("none");

  const paletteActions = useMemo(() => {
    const actions: Array<{
      id: string;
      label: string;
      description?: string;
      action: () => boolean | void;
    }> = [];

    actions.push({
      id: "switch-theme",
      label: "Switch Theme",
      description: `Current: ${themeStore.getActive()}`,
      action: () => {
        setOverlay("themes");
        return true;
      },
    });

    actions.push({
      id: "reload-config",
      label: "Reload Config",
      description: "Re-read frost.json",
      action: () => {
        setOverlay("none");
      },
    });

    return actions;
  }, [themeStore]);

  const searchResults = useMemo(() => {
    const results: Array<{ id: string; label: string; description?: string }> = [];
    for (const [projectName, project] of Object.entries(config.projects)) {
      results.push({
        id: `project:${projectName}`,
        label: projectName,
        description: "Project",
      });
      for (const appName of Object.keys(project.apps)) {
        results.push({
          id: `app:${projectName}/${appName}`,
          label: `${projectName}/${appName}`,
          description: proc.getStatus(projectName, appName),
        });
      }
    }
    return results;
  }, [config, proc]);

  const logTitle =
    nav.selection.appName && nav.selection.projectName
      ? `Logs: ${nav.selection.projectName}/${nav.selection.appName}`
      : "Logs";

  const selectedLogs =
    nav.selection.appName && nav.selection.projectName
      ? proc.getLogs(nav.selection.projectName, nav.selection.appName)
      : [];

  const selectedAppLabel: string | null =
    nav.selection.appName && nav.selection.projectName
      ? `${nav.selection.projectName}/${nav.selection.appName}`
      : null;

  useKeyboard((key) => {
    if (overlay !== "none") {
      if (key.name === "escape") setOverlay("none");
      return;
    }

    switch (key.name) {
      case "up":
        if (nav.focus === "sidebar") nav.moveUp();
        break;
      case "down":
        if (nav.focus === "sidebar") nav.moveDown();
        break;
      case "tab":
        nav.toggleFocus();
        break;
      case "return": {
        if (nav.selection.projectName) {
          if (nav.selection.appName) {
            void proc.toggleApp(nav.selection.projectName, nav.selection.appName);
          } else {
            proc.toggleProject(nav.selection.projectName);
          }
        }
        break;
      }
      case "r": {
        if (nav.selection.appName && nav.selection.projectName) {
          void proc.restartApp(nav.selection.projectName, nav.selection.appName);
        }
        break;
      }
      case "s": {
        if (nav.selection.projectName) {
          if (nav.selection.appName) {
            void proc.toggleApp(nav.selection.projectName, nav.selection.appName);
          } else {
            void proc.toggleAll(nav.selection.projectName);
          }
        }
        break;
      }
    }
  });

  useKeyboard((key) => {
    if (key.ctrl && key.name === "p") {
      setOverlay("palette");
      return;
    }
    if (key.name === "/" && overlay === "none") {
      setOverlay("search");
      return;
    }
    if (key.name === "escape") {
      if (overlay !== "none") {
        setOverlay("none");
      }
    }
  });

  return (
    <box>
      <box flexGrow={1} flexDirection="row">
        <Sidebar
          config={config}
          selection={nav.selection}
          getStatus={proc.getStatus}
          resolvedTheme={resolvedTheme}
        />

        <LogViewer logs={selectedLogs} title={logTitle} />
      </box>

      <CommandBar runningCount={proc.runningCount} selectedApp={selectedAppLabel} />

      {overlay !== "none" && (
        <box
          position="absolute"
          width="100%"
          height="100%"
          alignItems="center"
          justifyContent="center"
        >
          {overlay === "palette" && (
            <CommandPalette
              actions={paletteActions}
              onClose={() => setOverlay("none")}
              resolvedTheme={resolvedTheme}
            />
          )}
          {overlay === "themes" && (
            <ThemeDialog onClose={() => setOverlay("none")} resolvedTheme={resolvedTheme} />
          )}
          {overlay === "search" && (
            <SearchDialog
              results={searchResults}
              onClose={() => setOverlay("none")}
              onSelect={(id) => {
                if (id.startsWith("project:")) {
                  const projectName = id.slice(8);
                  nav.setSelection({ projectName, appName: null });
                } else if (id.startsWith("app:")) {
                  const parts = id.slice(4).split("/");
                  if (parts.length === 2 && parts[0] && parts[1]) {
                    nav.setSelection({ projectName: parts[0], appName: parts[1] });
                  }
                }
                setOverlay("none");
              }}
              resolvedTheme={resolvedTheme}
            />
          )}
        </box>
      )}
    </box>
  );
}
