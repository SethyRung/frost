// For TypeScript autocompletion, import defineConfig from "frost"
// At runtime, we inject defineConfig and capture the config value.
import { defineConfig } from "frost";

export default defineConfig({
  projects: {
    "my-web-app": {
      root: "./apps/web",
      apps: {
        frontend: {
          command: "bun dev",
          cwd: "./frontend",
        },
        api: {
          command: "bun start",
          cwd: "./api",
        },
      },
    },
  },
});
