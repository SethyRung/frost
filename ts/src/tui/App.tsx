import type { FrostConfig } from "@/config/types";
import type { ProcessManager } from "@/process/manager";
import { ThemeProvider } from "@/tui/theme";
import { Dashboard } from "./Dashboard";
import { Error } from "./components/Error";

interface AppProps {
  config: FrostConfig | null;
  configError: string | null;
  processManager: ProcessManager | null;
}

export function App({ config, configError, processManager }: AppProps) {
  if (configError) {
    return <Error error={configError} />;
  }

  if (!config || !processManager) {
    return <Error error="Create frost.json in your current directory or a parent directory." />;
  }

  return (
    <ThemeProvider>
      <Dashboard config={config} processManager={processManager} />
    </ThemeProvider>
  );
}
