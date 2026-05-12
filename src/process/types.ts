export type ProcessStatus = "stopped" | "starting" | "running" | "stopping" | "crashed";

export interface LogLine {
  timestamp: number;
  stream: "stdout" | "stderr";
  text: string;
}

export interface ProcessInfo {
  id: string;
  pid: number | null;
  command: string;
  cwd: string;
  status: ProcessStatus;
  logs: LogLine[];
}

export interface ProcessState {
  apps: Record<string, ProcessInfo>;
}
