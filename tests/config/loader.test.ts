import { describe, it, expect } from "bun:test";
import { findConfig, loadConfig, ConfigError } from "../../src/config/loader";

const PROJECT_ROOT = import.meta.dirname.replace(/\/tests\/config$/, "");

const FIXTURE_PATH = `${PROJECT_ROOT}/tests/fixtures/.frost.config.ts`;
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
    expect(config.projects).toBeDefined();
    const project = config.projects["my-web-app"];
    expect(project).toBeDefined();
    expect(project?.apps.frontend).toBeDefined();
    expect(project?.apps.frontend?.command).toBe("bun run dev");
    expect(project?.apps.api?.command).toBe("bun run start");
  });

  it("throws ConfigError for empty command", async () => {
    const badPath = "/tmp/bad-frost-config.ts";
    await Bun.write(
      badPath,
      `import { defineConfig } from '${PROJECT_ROOT}/src/config/define-config.ts';
export default defineConfig({ projects: { p: { apps: { a: { command: "" } } } } });`,
    );
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
    const badPath = "/tmp/bad-frost-config.ts";
    await Bun.write(
      badPath,
      `import { defineConfig } from '${PROJECT_ROOT}/src/config/define-config.ts';
export default defineConfig({});`,
    );
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
});
