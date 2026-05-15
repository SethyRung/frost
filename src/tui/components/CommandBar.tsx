import { TextAttributes } from "@opentui/core";

interface CommandBarProps {
  runningCount: number;
  selectedApp?: string | null;
}

const commands = [
  "\u2191/\u2193 Navigate",
  "s Start/Stop",
  "r Restart",
  "Tab Focus",
  "Ctrl+P Palette",
  "/ Search",
];

export function CommandBar({ runningCount, selectedApp }: CommandBarProps) {
  return (
    <box flexDirection="row" alignItems="center" borderStyle="rounded" paddingX={1}>
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
