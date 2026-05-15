import { useMemo, useState, useCallback, useEffect, useRef } from "react";
import { TextAttributes } from "@opentui/core";
import { useKeyboard } from "@opentui/react";
import { useThemeStore, useResolvedTheme } from "./provider";
import { rgbaToString } from "./resolver";

interface DialogThemeListProps {
  onClose: () => void;
  onSelect?: (id: string) => void;
}

export function DialogThemeList({ onClose, onSelect }: DialogThemeListProps) {
  const store = useThemeStore();
  const currentResolved = useResolvedTheme();
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);
  const [, setConfirmed] = useState(false);
  const initialTheme = useRef(store.getActive());

  const bg = currentResolved ? rgbaToString(currentResolved.background) : undefined;
  const cursorBg = currentResolved ? rgbaToString(currentResolved.backgroundElement) : undefined;
  const borderColor = currentResolved ? rgbaToString(currentResolved.border) : undefined;

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
    const target = query.trim() ? 0 : Math.max(0, allIds.indexOf(initialTheme.current));
    setCursor(target);
  }, [filteredIds.length]);

  useEffect(() => {
    const id = filteredIds[cursor];
    if (id && store.has(id)) {
      store.set(id);
    }
  }, [cursor, filteredIds, store]);

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
      borderStyle="single"
      paddingX={1}
      paddingY={1}
      width={60}
      style={{ borderColor, backgroundColor: bg }}
    >
      <text attributes={TextAttributes.BOLD}>Select Theme</text>
      <box paddingY={1}>
        <input placeholder="Filter themes..." onInput={handleInput} />
      </box>
      <box overflow="scroll" flexGrow={1}>
        {filteredIds.map((id, i) => (
          <ThemeListItem key={id} id={id} selected={i === cursor} cursorBg={cursorBg} />
        ))}
        {filteredIds.length === 0 && (
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
