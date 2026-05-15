import { useState, useEffect, useMemo } from "react";
import { TextAttributes } from "@opentui/core";
import { useKeyboard } from "@opentui/react";
import type { ResolvedTheme } from "@/tui/theme/types";
import { rgbaToString } from "@/tui/theme/resolver";

interface SearchResult {
  id: string;
  label: string;
  description?: string;
}

interface SearchDialogProps {
  results: SearchResult[];
  onClose: () => void;
  onSelect: (id: string) => void;
  resolvedTheme?: ResolvedTheme | null;
}

export function SearchDialog({ results, onClose, onSelect, resolvedTheme }: SearchDialogProps) {
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);

  const bg = resolvedTheme ? rgbaToString(resolvedTheme.background) : undefined;
  const cursorBg = resolvedTheme ? rgbaToString(resolvedTheme.backgroundElement) : undefined;

  const filtered = useMemo(() => {
    if (!query.trim()) return results;
    const q = query.toLowerCase();
    return results.filter(
      (r) => r.label.toLowerCase().includes(q) || r.description?.toLowerCase().includes(q),
    );
  }, [results, query]);

  useEffect(() => {
    setCursor(0);
  }, [filtered.length]);

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
      const result = filtered[cursor];
      if (result) {
        onSelect(result.id);
      }
      onClose();
      return;
    }
  });

  return (
    <box width={70} backgroundColor={bg} borderStyle="rounded" paddingX={1} paddingY={0.5}>
      <box flexDirection="row">
        <text attributes={TextAttributes.BOLD}>Search</text>
        <box flexGrow={1} />
        <text attributes={TextAttributes.DIM}>esc</text>
      </box>

      <box paddingY={1}>
        <input
          placeholder="Search projects and apps..."
          onInput={(v) => {
            setQuery(v);
            setCursor(0);
          }}
        />
      </box>

      <box overflow="scroll" flexGrow={1}>
        {filtered.map((result, i) => (
          <box
            key={result.id}
            flexDirection="row"
            paddingX={1}
            style={{
              backgroundColor: i === cursor ? cursorBg : undefined,
            }}
          >
            <text attributes={i === cursor ? TextAttributes.BOLD : undefined}>
              {i === cursor ? "\u203A " : "  "}
              {result.label}
            </text>
            {result.description ? <box flexGrow={1} /> : null}
            {result.description ? (
              <text attributes={TextAttributes.DIM}>{result.description}</text>
            ) : null}
          </box>
        ))}
        {filtered.length === 0 && <text attributes={TextAttributes.DIM}>No matches</text>}
      </box>
    </box>
  );
}
