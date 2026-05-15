import { parseAnsi, segmentToAttributes } from "@/tui/lib/ansi";

interface AnsiTextProps {
  text: string;
  defaultFg?: string;
  defaultBg?: string;
}

export function AnsiText({ text, defaultFg, defaultBg }: AnsiTextProps) {
  const segments = parseAnsi(text);

  if (segments.length === 0) {
    return (
      <text fg={defaultFg} bg={defaultBg}>
        {text}
      </text>
    );
  }

  if (segments.length === 1) {
    const seg = segments[0]!;
    return (
      <text
        fg={seg.fg ?? defaultFg}
        bg={seg.bg ?? defaultBg}
        attributes={segmentToAttributes(seg) || undefined}
      >
        {seg.text}
      </text>
    );
  }

  return (
    <box flexDirection="row">
      {segments.map((seg, i) => (
        <text
          key={i}
          fg={seg.fg ?? defaultFg}
          bg={seg.bg ?? defaultBg}
          attributes={segmentToAttributes(seg) || undefined}
        >
          {seg.text}
        </text>
      ))}
    </box>
  );
}
