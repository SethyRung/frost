import { useState, useEffect, useCallback, useRef } from "react";
import type { ProcessManager } from "@/process/manager";
import type { LogLine, ProcessStatus } from "@/process/types";
import type { FrostConfig } from "@/config/types";

export interface FlatAppItem {
  projectName: string;
  appName: string;
  projectRoot?: string;
  command: string;
  cwd?: string;
}

export interface AppProcessState {
  status: ProcessStatus;
  logs: LogLine[];
}

export interface ExpandedState {
  [projectName: string]: boolean;
}

export function useProcessManager(
  processManager: ProcessManager | null,
  config: FrostConfig | null,
) {
  const [processStates, setProcessStates] = useState<Record<string, AppProcessState>>({});
  const [expanded, setExpanded] = useState<ExpandedState>({});
  const statesRef = useRef(processStates);
  statesRef.current = processStates;

  const flatApps = useCallback((): FlatAppItem[] => {
    if (!config) return [];
    const items: FlatAppItem[] = [];
    for (const [projectName, project] of Object.entries(config.projects)) {
      for (const [appName, app] of Object.entries(project.apps)) {
        items.push({
          projectName,
          appName,
          projectRoot: project.root,
          command: app.command,
          cwd: app.cwd,
        });
      }
    }
    return items;
  }, [config]);

  const appId = useCallback(
    (projectName: string, appName: string): string => {
      return `${projectName}/${appName}`;
    },
    [],
  );

  const getStatus = useCallback(
    (projectName: string, appName: string): ProcessStatus => {
      const id = appId(projectName, appName);
      return processStates[id]?.status ?? "stopped";
    },
    [processStates, appId],
  );

  const getLogs = useCallback(
    (projectName: string, appName: string): LogLine[] => {
      const id = appId(projectName, appName);
      return processStates[id]?.logs ?? [];
    },
    [processStates, appId],
  );

  const toggleProject = useCallback(
    (projectName: string) => {
      setExpanded((prev) => ({
        ...prev,
        [projectName]: !prev[projectName],
      }));
    },
    [],
  );

  const isExpanded = useCallback(
    (projectName: string): boolean => {
      return expanded[projectName] ?? true;
    },
    [expanded],
  );

  useEffect(() => {
    if (!processManager || !config) return;

    // Initialize expanded state
    const initialExpanded: ExpandedState = {};
    for (const projectName of Object.keys(config.projects)) {
      initialExpanded[projectName] = true;
    }
    setExpanded(initialExpanded);

    // Subscribe to process events
    const handleLog = (appId: string, line: LogLine) => {
      setProcessStates((prev) => {
        const existing = prev[appId] ?? { status: "stopped", logs: [] };
        return {
          ...prev,
          [appId]: {
            ...existing,
            logs: [...existing.logs, line],
          },
        };
      });
    };

    const handleStateChange = (appId: string, status: ProcessStatus) => {
      setProcessStates((prev) => {
        const existing = prev[appId] ?? { status: "stopped", logs: [] };
        return {
          ...prev,
          [appId]: { ...existing, status },
        };
      });
    };

    processManager.on("log", handleLog);
    processManager.on("stateChange", handleStateChange);

    return () => {
      processManager.off("log", handleLog);
      processManager.off("stateChange", handleStateChange);
    };
  }, [processManager, config]);

  const startApp = useCallback(
    async (projectName: string, appName: string) => {
      if (!processManager || !config) return;
      const project = config.projects[projectName];
      if (!project) return;
      const app = project.apps[appName];
      if (!app) return;
      const id = appId(projectName, appName);
      const cwd = app.cwd
        ? project.root
          ? `${project.root}/${app.cwd}`
          : app.cwd
        : project.root;
      await processManager.start(id, app.command, cwd ?? process.cwd());
    },
    [processManager, config, appId],
  );

  const stopApp = useCallback(
    async (projectName: string, appName: string) => {
      if (!processManager) return;
      const id = appId(projectName, appName);
      await processManager.stop(id);
    },
    [processManager, appId],
  );

  const restartApp = useCallback(
    async (projectName: string, appName: string) => {
      if (!processManager) return;
      const id = appId(projectName, appName);
      await processManager.restart(id);
    },
    [processManager, appId],
  );

  const isActive = useCallback(
    (status: ProcessStatus): boolean => {
      return status === "running" || status === "starting" || status === "stopping";
    },
    [],
  );

  const toggleApp = useCallback(
    async (projectName: string, appName: string) => {
      const status = getStatus(projectName, appName);
      if (isActive(status)) {
        await stopApp(projectName, appName);
      } else {
        await startApp(projectName, appName);
      }
    },
    [getStatus, isActive, startApp, stopApp],
  );

  const startAll = useCallback(
    async (projectName: string) => {
      if (!config) return;
      const project = config.projects[projectName];
      if (!project) return;
      for (const appName of Object.keys(project.apps)) {
        await startApp(projectName, appName);
      }
    },
    [config, startApp],
  );

  const stopAll = useCallback(
    async (projectName: string) => {
      if (!config) return;
      const project = config.projects[projectName];
      if (!project) return;
      for (const appName of Object.keys(project.apps)) {
        await stopApp(projectName, appName);
      }
    },
    [config, stopApp],
  );

  const toggleAll = useCallback(
    async (projectName: string) => {
      if (!config) return;
      const project = config.projects[projectName];
      if (!project) return;
      const appNames = Object.keys(project.apps);
      const anyActive = appNames.some(
        (appName) => isActive(getStatus(projectName, appName)),
      );
      if (anyActive) {
        await stopAll(projectName);
      } else {
        await startAll(projectName);
      }
    },
    [config, getStatus, isActive, startAll, stopAll],
  );

  const runningCount = Object.values(processStates).filter(
    (s) => s.status === "running",
  ).length;

  return {
    processStates,
    flatApps,
    appId,
    getStatus,
    getLogs,
    toggleProject,
    isExpanded,
    expanded,
    startApp,
    stopApp,
    restartApp,
    toggleApp,
    startAll,
    stopAll,
    toggleAll,
    runningCount,
  };
}
