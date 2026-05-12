/**
 * Configuration for one app inside a project.
 */
export interface AppConfig {
  /** Command used to start the app, for example `bun run dev`. */
  command: string;
  /** Optional working directory for the app, relative to the project root. */
  cwd?: string;
}

/**
 * Configuration for a project grouping one or more apps.
 */
export interface ProjectConfig {
  /** Optional base directory for apps in this project, relative to the config file. */
  root?: string;
  /** Apps managed under this project, keyed by app name. */
  apps: Record<string, AppConfig>;
}

/**
 * Root Frost config shape loaded from `frost.config.ts`.
 */
export interface FrostConfig {
  /** All configured projects, keyed by project name. */
  projects: Record<string, ProjectConfig>;
}
