import type { LogLine } from "./types";

export interface SpawnOptions {
  command: string;
  cwd?: string;
}

export interface SpawnResult {
  pid: number;
  stdout: ReadableStream<Uint8Array> | null;
  stderr: ReadableStream<Uint8Array> | null;
  exitCode: Promise<number | null>;
  kill(): void;
}

export function spawnApp(opts: SpawnOptions): SpawnResult {
  const { command, cwd } = opts;

  const child = Bun.spawn({
    cmd: ["setsid", "sh", "-c", command],
    cwd,
    stdout: "pipe",
    stderr: "pipe",
    env: {
      ...process.env,
      FORCE_COLOR: "3",
      COLORTERM: "truecolor",
      TERM: "xterm-256color",
    },
  });

  return {
    pid: child.pid,
    stdout: child.stdout,
    stderr: child.stderr,
    exitCode: child.exited,
    kill: () => {
      try {
        process.kill(-child.pid, "SIGTERM");
      } catch {
        try {
          child.kill("SIGTERM");
        } catch {
          // already dead
        }
      }
    },
  };
}

export async function* readLines(
  stream: ReadableStream<Uint8Array> | null,
  encoding = "utf-8",
): AsyncGenerator<string> {
  if (!stream) return;
  const reader = stream.getReader();
  const decoder = new TextDecoder(encoding);
  let buffer = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) {
      if (buffer) yield buffer;
      break;
    }
    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split("\n");
    buffer = lines.pop() ?? "";
    for (const line of lines) {
      if (line) yield line;
    }
  }
}

export function makeLogLine(stream: "stdout" | "stderr", text: string): LogLine {
  return { timestamp: Date.now(), stream, text };
}

export const MAX_LOG_LINES = 1000;

export function appendLog(logs: LogLine[], line: LogLine): LogLine[] {
  const result = [...logs, line];
  if (result.length > MAX_LOG_LINES) {
    return result.slice(-MAX_LOG_LINES);
  }
  return result;
}
