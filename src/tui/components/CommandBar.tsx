import { TextAttributes } from "@opentui/core";

interface CommandBarProps {
  runningCount: number;
  selectedApp?: string | null;
  isCommandMode?: boolean;
  onCommandInput?: (value: string) => void;
  onCommandSubmit?: (value: string) => void;
}

export function CommandBar({
  runningCount,
  selectedApp,
  isCommandMode,
  onCommandInput,
  onCommandSubmit,
}: CommandBarProps) {
  return (
    <box flexDirection="row" alignItems="center" borderStyle="rounded" paddingX={1}>
      {isCommandMode ? (
        <>
          <text attributes={TextAttributes.BOLD}>&nbsp;[CMD]</text>
          <text>&nbsp;:</text>
          <box flexGrow={1}>
            <input
              focused
              placeholder="command"
              onInput={onCommandInput}
              onSubmit={(v) => {
                const val = typeof v === "string" ? v : "";
                onCommandSubmit?.(val);
              }}
            />
          </box>
          <text>
            &nbsp;{runningCount} Running{selectedApp ? ` \u2022 ${selectedApp}` : " \u2022 Ready"}
            &nbsp;
          </text>
        </>
      ) : (
        <>
          <text attributes={TextAttributes.DIM}>
            Enter:Toggle&nbsp;&nbsp;r:Restart&nbsp;&nbsp;p:Start
            Project&nbsp;&nbsp;/:Search&nbsp;&nbsp;:Command&nbsp;&nbsp;Ctrl+P:Palette
          </text>
          <box flexGrow={1} />
          <text>
            &nbsp;{runningCount} Running{selectedApp ? ` \u2022 ${selectedApp}` : " \u2022 Ready"}
            &nbsp;
          </text>
        </>
      )}
    </box>
  );
}
