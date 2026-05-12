import { TextAttributes } from "@opentui/core";

import type { FrostConfig } from "@/config/types";
import type { ProcessManager } from "@/process/manager";

interface AppProps {
  config: FrostConfig | null;
  configError: string | null;
  processManager: ProcessManager | null;
}

export function App({ config, configError, processManager }: AppProps) {
  return (
    <box alignItems="center" justifyContent="center" flexGrow={1}>
      <box justifyContent="center" alignItems="flex-end">
        <ascii-font font="tiny" text="OpenTUI" />
        <text attributes={TextAttributes.DIM}>What will you build?</text>
      </box>
    </box>
  );
}
