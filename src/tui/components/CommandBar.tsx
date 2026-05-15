import { TextAttributes } from "@opentui/core";
import type { ResolvedTheme } from "@/tui/theme/types";
import { rgbaToString } from "@/tui/theme";

interface CommandBarProps {
  runningCount: number;
  selectedApp?: string | null;
  resolvedTheme?: ResolvedTheme | null;
}

const commands = [
  "\u2191/\u2193 Navigate",
  "s Start/Stop",
  "r Restart",
  "Tab Focus",
  "Ctrl+P Palette",
  "/ Search",
];

export function CommandBar({ runningCount, selectedApp, resolvedTheme }: CommandBarProps) {
  const panelBg = resolvedTheme ? rgbaToString(resolvedTheme.backgroundPanel) : undefined;
  const borderColor = resolvedTheme ? rgbaToString(resolvedTheme.border) : undefined;

  return (
    <box
      flexDirection="row"
      alignItems="center"
      paddingX={1}
      borderStyle="rounded"
      backgroundColor={panelBg}
      borderColor={borderColor}
    >
      <box flexDirection="row" gap={4}>
        {commands.map((command) => (
          <text key={command} attributes={TextAttributes.DIM}>
            {command}
          </text>
        ))}
      </box>
      <box flexGrow={1} />
      <text>
        {runningCount} Running{selectedApp ? ` \u2022 ${selectedApp}` : " \u2022 Ready"}
      </text>
    </box>
  );
}
