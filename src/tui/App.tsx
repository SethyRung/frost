import { TextAttributes } from "@opentui/core";

import type { FrostConfig } from "@/config/types";
import type { ProcessManager } from "@/process/manager";

interface AppProps {
  config: FrostConfig | null;
  configError: string | null;
  processManager: ProcessManager | null;
}

export function App({ config, configError, processManager: _processManager }: AppProps) {
  if (configError) {
    return (
      <box alignItems="center" justifyContent="center" flexGrow={1} flexDirection="column" gap={1}>
        <text attributes={TextAttributes.BOLD}>Frost failed to load config</text>
        <text attributes={TextAttributes.DIM}>{configError}</text>
      </box>
    );
  }

  if (!config) {
    return (
      <box alignItems="center" justifyContent="center" flexGrow={1} flexDirection="column" gap={1}>
        <text attributes={TextAttributes.BOLD}>No frost.json found</text>
        <text attributes={TextAttributes.DIM}>
          Create frost.json in your current directory or a parent directory.
        </text>
      </box>
    );
  }

  return (
    <box alignItems="center" justifyContent="center" flexGrow={1}>
      <box justifyContent="center" alignItems="flex-end">
        <ascii-font font="tiny" text="OpenTUI" />
        <text attributes={TextAttributes.DIM}>What will you build?</text>
      </box>
    </box>
  );
}
