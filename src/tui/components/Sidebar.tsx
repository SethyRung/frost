import { TextAttributes } from "@opentui/core";
import type { FrostConfig } from "@/config/types";
import type { ProcessStatus } from "@/process/types";
import type { ResolvedTheme } from "@/tui/theme/types";
import { rgbaToString } from "@/tui/theme/resolver";
import { StatusIcon } from "./StatusIcon";

interface SidebarProps {
  config: FrostConfig;
  selection: { projectName: string; appName: string | null };
  getStatus: (projectName: string, appName: string) => ProcessStatus;
  resolvedTheme?: ResolvedTheme | null;
}

export function Sidebar({ config, selection, getStatus, resolvedTheme }: SidebarProps) {
  const selectBg = resolvedTheme ? rgbaToString(resolvedTheme.backgroundElement) : undefined;
  const panelBg = resolvedTheme ? rgbaToString(resolvedTheme.backgroundPanel) : undefined;
  const borderColor = resolvedTheme ? rgbaToString(resolvedTheme.border) : undefined;

  const projectNames = Object.keys(config.projects);

  return (
    <box width={30} borderStyle="rounded" backgroundColor={panelBg} borderColor={borderColor}>
      <box paddingX={1}>
        <text>Projects & Apps</text>
      </box>

      <box border={["top"]} borderColor={borderColor} />

      <box paddingX={1} gap={0.5}>
        {projectNames.length === 0 && (
          <text attributes={TextAttributes.DIM}>No projects configured</text>
        )}

        {projectNames.map((projectName) => {
          const project = config.projects[projectName]!;
          const appNames = Object.keys(project.apps);
          const isProjectSelected =
            selection.projectName === projectName && selection.appName === null;

          return (
            <box key={projectName}>
              <box
                paddingX={1}
                style={{
                  backgroundColor: isProjectSelected ? selectBg : undefined,
                }}
              >
                <text attributes={TextAttributes.BOLD}>{projectName}</text>
              </box>

              <box paddingLeft={2}>
                {appNames.map((appName) => {
                  const status = getStatus(projectName, appName);
                  const isAppSelected =
                    selection.projectName === projectName && selection.appName === appName;

                  return (
                    <box
                      key={appName}
                      flexDirection="row"
                      paddingX={1}
                      gap={1}
                      style={{
                        backgroundColor: isAppSelected ? selectBg : undefined,
                      }}
                    >
                      <StatusIcon status={status} />
                      <text>{appName}</text>
                    </box>
                  );
                })}
              </box>
            </box>
          );
        })}
      </box>
    </box>
  );
}
