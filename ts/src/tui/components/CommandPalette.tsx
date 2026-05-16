import { useState, useCallback, useEffect, useMemo } from "react";
import { TextAttributes } from "@opentui/core";
import { useKeyboard } from "@opentui/react";
import type { ResolvedTheme } from "@/tui/theme/types";
import { rgbaToString } from "@/tui/theme/resolver";

interface CommandPaletteAction {
  id: string;
  label: string;
  description?: string;
  action: () => boolean | void;
}

interface CommandPaletteProps {
  actions: CommandPaletteAction[];
  onClose: () => void;
  onSelect?: (id: string) => void;
  resolvedTheme?: ResolvedTheme | null;
}

export function CommandPalette({ actions, onClose, onSelect, resolvedTheme }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);

  const panelBg = resolvedTheme ? rgbaToString(resolvedTheme.backgroundPanel) : undefined;
  const borderColor = resolvedTheme ? rgbaToString(resolvedTheme.border) : undefined;
  const cursorBg = resolvedTheme ? rgbaToString(resolvedTheme.backgroundElement) : undefined;

  const filtered = useMemo(() => {
    if (!query.trim()) return actions;
    const q = query.toLowerCase();
    return actions.filter(
      (a) =>
        a.label.toLowerCase().includes(q) ||
        a.id.toLowerCase().includes(q) ||
        a.description?.toLowerCase().includes(q),
    );
  }, [actions, query]);

  useEffect(() => {
    setCursor(0);
  }, [filtered.length]);

  const selectCurrent = useCallback(() => {
    const action = filtered[cursor];
    if (action) {
      onSelect?.(action.id);
      const handled = action.action();
      if (handled) return;
    }
    onClose();
  }, [filtered, cursor, onSelect, onClose]);

  useKeyboard((key) => {
    if (key.name === "escape") {
      onClose();
      return;
    }
    if (key.name === "down" || (key.name === "tab" && !key.shift)) {
      setCursor((p) => Math.min(p + 1, filtered.length - 1));
      return;
    }
    if (key.name === "up" || (key.name === "tab" && key.shift)) {
      setCursor((p) => Math.max(p - 1, 0));
      return;
    }
    if (key.name === "return") {
      selectCurrent();
      return;
    }
  });

  return (
    <box
      width={70}
      paddingX={1}
      paddingY={0.5}
      borderStyle="rounded"
      backgroundColor={panelBg}
      borderColor={borderColor}
    >
      <box flexDirection="row">
        <text attributes={TextAttributes.BOLD}>Commands</text>
        <box flexGrow={1} />
        <text attributes={TextAttributes.DIM}>esc</text>
      </box>

      <box paddingY={1}>
        <input
          placeholder="Query..."
          onInput={(v) => {
            setQuery(v);
            setCursor(0);
          }}
        />
      </box>

      <box height={20} overflow="scroll" flexGrow={1}>
        {filtered.map((action, i) => (
          <box
            key={action.id}
            flexDirection="row"
            paddingX={1}
            style={{
              backgroundColor: i === cursor ? cursorBg : undefined,
            }}
          >
            <text attributes={i === cursor ? TextAttributes.BOLD : undefined}>
              {i === cursor ? "\u203A " : "  "}
              {action.label}
            </text>
            {action.description ? <box flexGrow={1} /> : null}
            {action.description ? (
              <text attributes={TextAttributes.DIM}>{action.description}</text>
            ) : null}
          </box>
        ))}
        {filtered.length === 0 && <text attributes={TextAttributes.DIM}>No matching commands</text>}
      </box>
    </box>
  );
}
