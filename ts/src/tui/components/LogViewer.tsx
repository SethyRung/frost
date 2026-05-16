import { TextAttributes } from "@opentui/core";
import type { LogLine } from "@/process/types";
import type { ResolvedTheme } from "@/tui/theme/types";
import { rgbaToString } from "@/tui/theme/resolver";
import { AnsiText } from "./AnsiText";

interface LogViewerProps {
  logs: LogLine[];
  title: string;
  resolvedTheme?: ResolvedTheme | null;
}

export function LogViewer({ logs, title, resolvedTheme }: LogViewerProps) {
  const panelBg = resolvedTheme ? rgbaToString(resolvedTheme.backgroundPanel) : undefined;
  const borderColor = resolvedTheme ? rgbaToString(resolvedTheme.border) : undefined;
  const titleColor = resolvedTheme ? rgbaToString(resolvedTheme.text) : undefined;
  const mutedColor = resolvedTheme ? rgbaToString(resolvedTheme.textMuted) : undefined;
  const stdoutColor = resolvedTheme ? rgbaToString(resolvedTheme.text) : undefined;
  const stderrColor = resolvedTheme ? rgbaToString(resolvedTheme.error) : undefined;

  return (
    <box flexGrow={1} borderStyle="rounded" backgroundColor={panelBg} borderColor={borderColor}>
      <box paddingX={1}>
        <text fg={titleColor}>{title}</text>
      </box>

      <box border={["top"]} borderColor={borderColor} />

      <scrollbox paddingX={1} overflow="scroll" flexGrow={1} stickyStart="bottom">
        {logs.length === 0 && (
          <text attributes={TextAttributes.DIM} fg={mutedColor}>
            No logs yet. Start an app to see output.
          </text>
        )}

        {logs.map((line, i) => (
          <AnsiText
            key={`${line.timestamp}-${i}`}
            text={line.text}
            defaultFg={line.stream === "stderr" ? stderrColor : stdoutColor}
          />
        ))}
      </scrollbox>
    </box>
  );
}
