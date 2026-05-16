import { readdir, readFile } from "fs/promises";
import { join } from "path";
import type { ThemeJson } from "./types";

function getGlobalThemesDir(): string {
  const xdg = process.env.XDG_CONFIG_HOME;
  const configDir = xdg ? join(xdg, "frost") : join(process.env.HOME ?? "/root", ".config", "frost");
  return join(configDir, "themes");
}

export function getThemeDirectories(): string[] {
  const dirs: string[] = [];
  const globalDir = getGlobalThemesDir();
  dirs.push(globalDir);

  // Discover project .frost/themes dirs by walking up from cwd
  const cwd = process.cwd();
  const parts = cwd.split("/").filter(Boolean);
  for (let i = parts.length; i >= 0; i--) {
    const path = "/" + parts.slice(0, i).join("/");
    dirs.push(join(path, ".frost", "themes"));
  }

  return [...new Set(dirs)];
}

async function loadThemesFromDir(dir: string): Promise<Record<string, ThemeJson>> {
  const themes: Record<string, ThemeJson> = {};
  try {
    const entries = await readdir(dir, { withFileTypes: true });
    const jsonFiles = entries.filter(
      (e) => e.isFile() && (e.name.endsWith(".json") || e.name.endsWith(".jsonc")),
    );

    for (const file of jsonFiles) {
      try {
        const content = await readFile(join(dir, file.name), "utf-8");
        const parsed = JSON.parse(content) as ThemeJson;
        if (parsed && typeof parsed === "object" && parsed.theme) {
          const id = file.name.replace(/\.(json|jsonc)$/, "");
          themes[id] = parsed;
        }
      } catch {
        // ignore invalid theme files
      }
    }
  } catch {
    // directory doesn't exist or can't be read
  }
  return themes;
}

export async function loadCustomThemes(): Promise<Record<string, ThemeJson>> {
  const dirs = getThemeDirectories();
  const allThemes: Record<string, ThemeJson> = {};

  // Process dirs in order - later dirs overwrite earlier ones with same id
  for (const dir of dirs) {
    const themes = await loadThemesFromDir(dir);
    for (const [id, theme] of Object.entries(themes)) {
      allThemes[id] = theme;
    }
  }

  return allThemes;
}
