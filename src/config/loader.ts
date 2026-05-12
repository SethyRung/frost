import type { FrostConfig } from "./types";

export class ConfigError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ConfigError";
  }
}

export async function findConfig(cwd: string = process.cwd()): Promise<string | null> {
  let dir = cwd;
  while (true) {
    const configPath = `${dir}/frost.config.ts`;
    try {
      await Bun.file(configPath).text();
      return configPath;
    } catch {
      // doesn't exist, continue
    }
    const parent = dir.replace(/\/[^/]*$/, "");
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

function validateConfig(obj: unknown): FrostConfig {
  if (typeof obj !== "object" || obj === null) {
    throw new ConfigError("Config must be an object");
  }
  const config = obj as Record<string, unknown>;
  if (!config.projects || typeof config.projects !== "object") {
    throw new ConfigError("Config must have a 'projects' object");
  }
  const projects = config.projects as Record<string, unknown>;
  for (const [name, project] of Object.entries(projects)) {
    if (typeof project !== "object" || project === null) {
      throw new ConfigError(`Project '${name}' must be an object`);
    }
    const p = project as Record<string, unknown>;
    if (!p.apps || typeof p.apps !== "object") {
      throw new ConfigError(`Project '${name}' must have an 'apps' object`);
    }
    const apps = p.apps as Record<string, unknown>;
    for (const [appName, app] of Object.entries(apps)) {
      if (typeof app !== "object" || app === null) {
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
  // Use Bun.$.ts() to evaluate the config file in a controlled context.
  // We inject defineConfig as identity so users can optionally use it for
  // better TypeScript inference, and we capture the default export.
  const code = await Bun.file(path).text();
  const lines = code.split("\n").filter((l) => !l.trim().startsWith("import "));
  const body = lines.join("\n");
  const tmp = `/tmp/frost-config-${Date.now()}-${Math.random()}.mjs`;
  const runner = `
let __cfg__ = undefined;
const defineConfig = (cfg) => { __cfg__ = cfg; return cfg; };
${body}
if (__cfg__ === undefined) throw new Error("No config found");
process.stdout.write(JSON.stringify(__cfg__));
`;
  await Bun.write(tmp, runner);
  try {
    const child = Bun.spawnSync({ cmd: ["bun", "run", tmp], stdout: "pipe", stderr: "pipe" });
    const out = new TextDecoder().decode(child.stdout);
    if (child.exitCode !== 0) {
      const err = new TextDecoder().decode(child.stderr);
      throw new ConfigError(`Config parse error: ${err || out}`);
    }
    return validateConfig(JSON.parse(out.trim()));
  } catch (e) {
    if (e instanceof ConfigError) throw e;
    throw new ConfigError(`Failed to load config: ${(e as Error).message}`);
  } finally {
    try {
      await Bun.file(tmp).delete();
    } catch {
      /* ignore */
    }
  }
}
