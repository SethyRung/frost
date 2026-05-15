import { TextAttributes } from "@opentui/core";
import type { LogLine } from "@/process/types";
import { AnsiText } from "./AnsiText";

interface LogViewerProps {
  logs: LogLine[];
  title: string;
}

export function LogViewer({ logs, title }: LogViewerProps) {
  return (
    <box flexGrow={1} borderStyle="rounded">
      <box paddingX={1}>
        <text>{title}</text>
      </box>

      <box border={["top"]} />

      <scrollbox paddingX={1} overflow="scroll" flexGrow={1} stickyStart="bottom">
        {logs.length === 0 && (
          <text attributes={TextAttributes.DIM}>No logs yet. Start an app to see output.</text>
        )}

        {logs.map((line, i) => (
          <AnsiText key={i} text={line.text} />
        ))}
      </scrollbox>
    </box>
  );
}
