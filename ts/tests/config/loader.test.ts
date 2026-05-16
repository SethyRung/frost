import { describe, it, expect } from "bun:test";
import { findConfig, loadConfig, ConfigError } from "../../src/config/loader";

const PROJECT_ROOT = import.meta.dirname.replace(/\/tests\/config$/, "");

const FIXTURE_PATH = `${PROJECT_ROOT}/tests/fixtures/frost.json`;
const FIXTURE_DIR = `${PROJECT_ROOT}/tests/fixtures`;

describe("findConfig", () => {
  it("returns path when config exists in cwd", async () => {
    const result = await findConfig(FIXTURE_DIR);
    expect(result).toBe(FIXTURE_PATH);
  });

  it("returns null when no config found", async () => {
    const result = await findConfig("/tmp");
    expect(result).toBeNull();
  });
});

describe("loadConfig", () => {
  it("loads and validates fixture config", async () => {
    const config = await loadConfig(FIXTURE_PATH);
    expect(config.$schema).toBe("./schemas/config.json");
    expect(config.projects).toBeDefined();
    const project = config.projects["my-web-app"];
    expect(project).toBeDefined();
    expect(project?.apps.frontend).toBeDefined();
    expect(project?.apps.frontend?.command).toBe("bun run dev");
    expect(project?.apps.api?.command).toBe("bun run start");
  });

  it("throws ConfigError for empty command", async () => {
    const badPath = `/tmp/bad-frost-config-${Date.now()}-empty-command.json`;
    await Bun.write(badPath, JSON.stringify({ projects: { p: { apps: { a: { command: "" } } } } }));
    try {
      await loadConfig(badPath);
      throw new Error("should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(ConfigError);
      expect((e as ConfigError).message).toContain("non-empty 'command'");
    } finally {
      try {
        Bun.file(badPath).delete();
      } catch {
        /* ignore */
      }
    }
  });

  it("throws ConfigError for missing projects", async () => {
    const badPath = `/tmp/bad-frost-config-${Date.now()}-missing-projects.json`;
    await Bun.write(badPath, JSON.stringify({}));
    try {
      await loadConfig(badPath);
      throw new Error("should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(ConfigError);
      expect((e as ConfigError).message).toContain("'projects'");
    } finally {
      try {
        Bun.file(badPath).delete();
      } catch {
        /* ignore */
      }
    }
  });

  it("loads jsonc config with comments and trailing commas", async () => {
    const jsoncPath = `/tmp/frost-config-${Date.now()}-jsonc.json`;
    await Bun.write(
      jsoncPath,
      `{
  // jsonc comment
  "$schema": "./schemas/config.json",
  "projects": {
    "p": {
      "apps": {
        "a": {
          "command": "bun run dev",
        },
      },
    },
  },
}`,
    );
    try {
      const config = await loadConfig(jsoncPath);
      expect(config.projects.p?.apps.a?.command).toBe("bun run dev");
    } finally {
      try {
        Bun.file(jsoncPath).delete();
      } catch {
        /* ignore */
      }
    }
  });
});
