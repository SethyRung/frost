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

  const handleInput = useCallback(
    (value: string) => {
      setQuery(value);
      setCursor(0);
    },
    [],
  );

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

  const borderColor = currentResolved ? rgbaToString(currentResolved.border) : undefined;

  return (
    <box
      borderStyle="single"
      padding={1}
      width={60}
      height={Math.min(filteredIds.length + 5, 20)}
      style={{ borderColor }}
    >
      <text attributes={TextAttributes.BOLD}>Select Theme</text>
      <box marginTop={1}>
        <input
          placeholder="Filter themes..."
          onInput={handleInput}
        />
      </box>
      <box marginTop={1} flexDirection="column" overflow="scroll">
        {filteredIds.map((id, i) => (
          <ThemeListItem
            key={id}
            id={id}
            selected={i === cursor}
          />
        ))}
        {filteredIds.length === 0 && (
          <text attributes={TextAttributes.DIM}>No themes match your query</text>
        )}
      </box>
      <box marginTop={1} gap={1}>
        <text attributes={TextAttributes.DIM}>&uarr;&darr; navigate</text>
        <text attributes={TextAttributes.DIM}>Enter select</text>
        <text attributes={TextAttributes.DIM}>Esc cancel</text>
      </box>
    </box>
  );
}

interface ThemeListItemProps {
  id: string;
  selected: boolean;
}

function ThemeListItem({ id, selected }: ThemeListItemProps) {
  return (
    <box
      paddingX={1}
      style={{
        backgroundColor: selected ? "#555" : undefined,
      }}
    >
      <text attributes={selected ? TextAttributes.BOLD : undefined}>{id}</text>
    </box>
  );
}
