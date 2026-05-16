import type { FrostConfig } from "./types";
import { type ParseError, parse as parseJsonc, printParseErrorCode } from "jsonc-parser";

export class ConfigError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ConfigError";
  }
}

export async function findConfig(cwd: string = process.cwd()): Promise<string | null> {
  const filenames = ["frost.json", "frost.jsonc"];
  let dir = cwd;
  while (true) {
    for (const filename of filenames) {
      const configPath = `${dir}/${filename}`;
      try {
        await Bun.file(configPath).text();
        return configPath;
      } catch {
        // doesn't exist, continue
      }
    }
    const parent = dir.replace(/\/[^/]*$/, "");
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

function parseConfig(text: string, path: string): unknown {
  const errors: ParseError[] = [];
  const parsed = parseJsonc(text, errors, { allowTrailingComma: true });
  if (!errors.length) return parsed;
  const messages = errors.map((err) => {
    const line = text.slice(0, err.offset).split("\n").length;
    const column = err.offset - text.lastIndexOf("\n", err.offset - 1);
    return `${printParseErrorCode(err.error)} at line ${line}, column ${column}`;
  });
  throw new ConfigError(`Config parse error in '${path}': ${messages.join("; ")}`);
}

function validateConfig(obj: unknown): FrostConfig {
  if (typeof obj !== "object" || obj === null) {
    throw new ConfigError("Config must be an object");
  }
  const config = obj as Record<string, unknown>;
  if (config.$schema !== undefined && typeof config.$schema !== "string") {
    throw new ConfigError("Config field '$schema' must be a string when provided");
  }
  if (!config.projects || typeof config.projects !== "object" || Array.isArray(config.projects)) {
    throw new ConfigError("Config must have a 'projects' object");
  }
  const projects = config.projects as Record<string, unknown>;
  for (const [name, project] of Object.entries(projects)) {
    if (typeof project !== "object" || project === null || Array.isArray(project)) {
      throw new ConfigError(`Project '${name}' must be an object`);
    }
    const p = project as Record<string, unknown>;
    if (!p.apps || typeof p.apps !== "object" || Array.isArray(p.apps)) {
      throw new ConfigError(`Project '${name}' must have an 'apps' object`);
    }
    const apps = p.apps as Record<string, unknown>;
    for (const [appName, app] of Object.entries(apps)) {
      if (typeof app !== "object" || app === null || Array.isArray(app)) {
        throw new ConfigError(`App '${appName}' in project '${name}' must be an object`);
      }
      const a = app as Record<string, unknown>;
      if (typeof a.command !== "string" || a.command.trim() === "") {
        throw new ConfigError(
          `App '${appName}' in project '${name}' must have a non-empty 'command' string`,
        );
      }
    }
  }
  return config as unknown as FrostConfig;
}

export async function loadConfig(path: string): Promise<FrostConfig> {
  try {
    const text = await Bun.file(path).text();
    return validateConfig(parseConfig(text, path));
  } catch (e) {
    if (e instanceof ConfigError) throw e;
    throw new ConfigError(`Failed to load config '${path}': ${(e as Error).message}`);
  }
}
