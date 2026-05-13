import { createCliRenderer } from "@opentui/core";
import { createRoot } from "@opentui/react";
import { App } from "@/tui/App";
import { findConfig, loadConfig } from "@/config/loader";
import type { FrostConfig } from "@/config/types";
import { ProcessManager } from "@/process/manager";

const renderer = await createCliRenderer({ exitOnCtrlC: true });

let config: FrostConfig | null = null;
let configError: string | null = null;

try {
  const configPath = await findConfig();
  if (configPath) {
    config = await loadConfig(configPath);
  } else {
    configError = "No frost.json found in current or parent directories.";
  }
} catch (e) {
  configError = e instanceof Error ? e.message : "Unknown config error";
}

const processManager = new ProcessManager();

const root = createRoot(renderer);
root.render(<App config={config} configError={configError} processManager={processManager} />);

function cleanup() {
  try {
    renderer.destroy();
  } catch {}
}

process.on("SIGINT", cleanup);
process.on("SIGTERM", cleanup);
