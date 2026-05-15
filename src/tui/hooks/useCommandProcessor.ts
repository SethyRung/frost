import { useCallback } from "react";

export type CommandHandler = (args: string[]) => void;

export interface CommandRegistry {
  [command: string]: CommandHandler;
}

export interface CommandProcessorResult {
  process(input: string): boolean;
  getSuggestions(input: string): string[];
}

export function useCommandProcessor(
  registry: CommandRegistry,
  onUnknown?: (cmd: string) => void,
): CommandProcessorResult {
  const process = useCallback(
    (input: string): boolean => {
      const trimmed = input.trim();
      if (!trimmed) return false;

      const parts = trimmed.split(/\s+/);
      const cmd = parts[0]!;
      const args = parts.slice(1);

      const handler = registry[cmd];
      if (handler) {
        handler(args);
        return true;
      }

      onUnknown?.(cmd);
      return false;
    },
    [registry, onUnknown],
  );

  const getSuggestions = useCallback(
    (input: string): string[] => {
      const trimmed = input.trim().toLowerCase();
      if (!trimmed) return Object.keys(registry);
      return Object.keys(registry).filter((cmd) =>
        cmd.toLowerCase().startsWith(trimmed),
      );
    },
    [registry],
  );

  return { process, getSuggestions };
}
