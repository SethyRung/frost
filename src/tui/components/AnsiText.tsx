import { parseAnsi, segmentToAttributes } from "@/tui/lib/ansi";

interface AnsiTextProps {
  text: string;
}

export function AnsiText({ text }: AnsiTextProps) {
  const segments = parseAnsi(text);

  if (segments.length === 0) {
    return <text>{text}</text>;
  }

  if (segments.length === 1) {
    const seg = segments[0]!;
    return (
      <text fg={seg.fg} bg={seg.bg} attributes={segmentToAttributes(seg) || undefined}>
        {seg.text}
      </text>
    );
  }

  return (
    <box flexDirection="row">
      {segments.map((seg, i) => (
        <text key={i} fg={seg.fg} bg={seg.bg} attributes={segmentToAttributes(seg) || undefined}>
          {seg.text}
        </text>
      ))}
    </box>
  );
}
