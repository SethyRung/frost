import { useMemo, useState, useCallback, useEffect, useRef } from "react";
import { TextAttributes } from "@opentui/core";
import { useKeyboard } from "@opentui/react";
import type { ResolvedTheme } from "@/tui/theme/types";
import { useThemeStore, useResolvedTheme } from "@/tui/theme/provider";
import { rgbaToString } from "@/tui/theme/resolver";

const VIEWPORT_SIZE = 10;

interface ThemeDialogProps {
  onClose: () => void;
  onSelect?: (id: string) => void;
  resolvedTheme?: ResolvedTheme | null;
}

export function ThemeDialog({ onClose, onSelect, resolvedTheme }: ThemeDialogProps) {
  const store = useThemeStore();
  const hookResolved = useResolvedTheme();
  const currentResolved = resolvedTheme ?? hookResolved;
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);
  const [scrollOffset, setScrollOffset] = useState(0);
  const [, setConfirmed] = useState(false);
  const initialTheme = useRef(store.getActive());

  const panelBg = resolvedTheme ? rgbaToString(resolvedTheme.backgroundPanel) : undefined;
  const borderColor = resolvedTheme ? rgbaToString(resolvedTheme.border) : undefined;
  const cursorBg = currentResolved ? rgbaToString(currentResolved.backgroundElement) : undefined;

  const allIds = useMemo(() => {
    return Object.keys(store.getAll()).sort((a, b) =>
      a.localeCompare(b, undefined, { sensitivity: "base" }),
    );
  }, [store]);

  const filteredIds = useMemo(() => {
    if (!query.trim()) return allIds;
    const q = query.toLowerCase();
    return allIds.filter((id) => id.toLowerCase().includes(q));
  }, [allIds, query]);

  useEffect(() => {
    const maxCursor = Math.max(0, filteredIds.length - 1);
    const target = query.trim()
      ? 0
      : Math.min(Math.max(0, allIds.indexOf(initialTheme.current)), maxCursor);
    setCursor(target);
  }, [filteredIds.length]);

  useEffect(() => {
    const id = filteredIds[cursor];
    if (id && store.has(id)) {
      store.set(id);
    }
  }, [cursor, filteredIds, store]);

  useEffect(() => {
    if (filteredIds.length <= VIEWPORT_SIZE) {
      setScrollOffset(0);
    } else if (cursor < scrollOffset) {
      setScrollOffset(cursor);
    } else if (cursor >= scrollOffset + VIEWPORT_SIZE) {
      setScrollOffset(cursor - VIEWPORT_SIZE + 1);
    }
  }, [cursor, filteredIds.length, scrollOffset]);

  const visibleIds = useMemo(() => {
    return filteredIds.slice(scrollOffset, scrollOffset + VIEWPORT_SIZE);
  }, [filteredIds, scrollOffset]);

  const visibleCursor = cursor - scrollOffset;

  const handleSelect = useCallback(() => {
    const id = filteredIds[cursor];
    if (id && store.has(id)) {
      store.set(id);
      setConfirmed(true);
      onSelect?.(id);
    }
    onClose();
  }, [cursor, filteredIds, store, onSelect, onClose]);

  const handleCancel = useCallback(() => {
    store.set(initialTheme.current);
    onClose();
  }, [store, onClose]);

  const handleInput = useCallback((value: string) => {
    setQuery(value);
    setCursor(0);
    setScrollOffset(0);
  }, []);

  useKeyboard((key) => {
    if (key.name === "down" || (key.name === "tab" && !key.shift)) {
      setCursor((p) => Math.min(p + 1, filteredIds.length - 1));
      return;
    }
    if (key.name === "up" || (key.name === "tab" && key.shift)) {
      setCursor((p) => Math.max(p - 1, 0));
      return;
    }
    if (key.name === "return") {
      handleSelect();
      return;
    }
    if (key.name === "escape") {
      handleCancel();
      return;
    }
  });

  return (
    <box
      width={70}
      borderStyle="rounded"
      backgroundColor={panelBg}
      borderColor={borderColor}
      paddingX={1}
      paddingY={0.5}
    >
      <text attributes={TextAttributes.BOLD}>Select Theme</text>
      <box paddingY={1}>
        <input placeholder="Filter themes..." onInput={handleInput} />
      </box>
      <box height={VIEWPORT_SIZE}>
        {visibleIds.map((id, i) => (
          <ThemeListItem key={id} id={id} selected={i === visibleCursor} cursorBg={cursorBg} />
        ))}
        {visibleIds.length === 0 && (
          <text attributes={TextAttributes.DIM}>No themes match your query</text>
        )}
      </box>
      <box paddingY={1} flexDirection="row">
        <text attributes={TextAttributes.DIM}>&uarr;&darr; navigate</text>
        <box flexGrow={1} />
        <text attributes={TextAttributes.DIM}>Enter select</text>
        <text attributes={TextAttributes.DIM}>&nbsp;Esc cancel</text>
      </box>
    </box>
  );
}

interface ThemeListItemProps {
  id: string;
  selected: boolean;
  cursorBg?: string;
}

function ThemeListItem({ id, selected, cursorBg }: ThemeListItemProps) {
  return (
    <box
      height={1}
      paddingX={1}
      style={{
        backgroundColor: selected ? cursorBg : undefined,
      }}
    >
      <text attributes={selected ? TextAttributes.BOLD : undefined}>
        {selected ? "\u203A " : "  "}
        {id}
      </text>
    </box>
  );
}
