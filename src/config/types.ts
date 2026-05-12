export interface AppConfig {
  command: string;
  cwd?: string;
}

export interface ProjectConfig {
  root?: string;
  apps: Record<string, AppConfig>;
}

export interface FrostConfig {
  projects: Record<string, ProjectConfig>;
}
