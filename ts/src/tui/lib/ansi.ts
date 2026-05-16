import { TextAttributes } from "@opentui/core";

interface AnsiSegment {
  text: string;
  fg?: string;
  bg?: string;
  bold?: boolean;
  dim?: boolean;
  italic?: boolean;
  underline?: boolean;
  blink?: boolean;
  inverse?: boolean;
  strikethrough?: boolean;
}

const ANSI_COLORS = [
  "#000000",
  "#FF0000",
  "#00FF00",
  "#FFFF00",
  "#0000FF",
  "#FF00FF",
  "#00FFFF",
  "#FFFFFF",
];

const BRIGHT_COLORS = [
  "#808080",
  "#FF5555",
  "#55FF55",
  "#FFFF55",
  "#5555FF",
  "#FF55FF",
  "#55FFFF",
  "#FFFFFF",
];

const XTERM_256 = (() => {
  const vals = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];
  const colors: string[] = [];
  for (const r of vals) {
    for (const g of vals) {
      for (const b of vals) {
        colors.push(
          `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`,
        );
      }
    }
  }
  for (let i = 0; i < 24; i++) {
    const gray = 8 + i * 10;
    const hex = gray.toString(16).padStart(2, "0");
    colors.push(`#${hex}${hex}${hex}`);
  }
  return colors;
})();

function toHex(n: number): string {
  return Math.max(0, Math.min(255, n)).toString(16).padStart(2, "0");
}

function get256Color(n: number): string {
  return XTERM_256[Math.max(0, Math.min(255, n))] ?? "#FFFFFF";
}

function applyCodes(codes: number[], style: AnsiSegment): AnsiSegment {
  if (codes.length === 0) {
    return { text: "" };
  }

  const result: AnsiSegment = { text: "" };
  if (style.fg) result.fg = style.fg;
  if (style.bg) result.bg = style.bg;
  if (style.bold) result.bold = style.bold;
  if (style.dim) result.dim = style.dim;
  if (style.italic) result.italic = style.italic;
  if (style.underline) result.underline = style.underline;
  if (style.blink) result.blink = style.blink;
  if (style.inverse) result.inverse = style.inverse;
  if (style.strikethrough) result.strikethrough = style.strikethrough;

  let i = 0;
  while (i < codes.length) {
    const code = codes[i];

    switch (code) {
      case 0: {
        // Reset all
        const cleared: AnsiSegment = { text: "" };
        Object.assign(result, cleared);
        break;
      }
      case 1:
        result.bold = true;
        break;
      case 2:
        result.dim = true;
        break;
      case 3:
        result.italic = true;
        break;
      case 4:
        result.underline = true;
        break;
      case 5:
        result.blink = true;
        break;
      case 7:
        result.inverse = true;
        break;
      case 9:
        result.strikethrough = true;
        break;
      case 22:
        result.bold = false;
        result.dim = false;
        break;
      case 23:
        result.italic = false;
        break;
      case 24:
        result.underline = false;
        break;
      case 25:
        result.blink = false;
        break;
      case 27:
        result.inverse = false;
        break;
      case 29:
        result.strikethrough = false;
        break;
      case 30:
      case 31:
      case 32:
      case 33:
      case 34:
      case 35:
      case 36:
      case 37:
        result.fg = ANSI_COLORS[code - 30];
        break;
      case 39:
        result.fg = undefined;
        break;
      case 40:
      case 41:
      case 42:
      case 43:
      case 44:
      case 45:
      case 46:
      case 47:
        result.bg = ANSI_COLORS[code - 40];
        break;
      case 49:
        result.bg = undefined;
        break;
      case 90:
      case 91:
      case 92:
      case 93:
      case 94:
      case 95:
      case 96:
      case 97:
        result.fg = BRIGHT_COLORS[code - 90];
        break;
      case 100:
      case 101:
      case 102:
      case 103:
      case 104:
      case 105:
      case 106:
      case 107:
        result.bg = BRIGHT_COLORS[code - 100];
        break;
      case 38:
        if (codes[i + 1] === 5 && i + 2 < codes.length) {
          result.fg = get256Color(codes[i + 2]!);
          i += 2;
        } else if (codes[i + 1] === 2 && i + 4 < codes.length) {
          result.fg = `#${toHex(codes[i + 2]!)}${toHex(codes[i + 3]!)}${toHex(codes[i + 4]!)}`;
          i += 4;
        }
        break;
      case 48:
        if (codes[i + 1] === 5 && i + 2 < codes.length) {
          result.bg = get256Color(codes[i + 2]!);
          i += 2;
        } else if (codes[i + 1] === 2 && i + 4 < codes.length) {
          result.bg = `#${toHex(codes[i + 2]!)}${toHex(codes[i + 3]!)}${toHex(codes[i + 4]!)}`;
          i += 4;
        }
        break;
    }

    i++;
  }

  return result;
}

const ESC = "\u001B";
const BEL = "\u0007";

const NON_SGR_ESCAPE = new RegExp(
  `${ESC}\\][^${BEL}${ESC}]*(?:${BEL}|${ESC}\\\\)|${ESC}\\[[\\d;]*[^\\dm]`,
  "g",
);

export function parseAnsi(input: string): AnsiSegment[] {
  const cleaned = input.replace(NON_SGR_ESCAPE, "");

  const segments: AnsiSegment[] = [];
  const regex = new RegExp(`${ESC}\\[([\\d;]*)m`, "g");
  let lastIndex = 0;
  let style: AnsiSegment = { text: "" };
  let match: RegExpExecArray | null;

  while ((match = regex.exec(cleaned)) !== null) {
    if (match.index > lastIndex) {
      segments.push({ ...style, text: cleaned.slice(lastIndex, match.index) });
    }

    const codes = match[1]!
      .split(";")
      .filter((s) => s !== "")
      .map(Number);
    style = applyCodes(codes, style);
    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < cleaned.length) {
    segments.push({ ...style, text: cleaned.slice(lastIndex) });
  }

  return segments.filter((s) => s.text !== "");
}

export function segmentToAttributes(segment: AnsiSegment): number {
  let attrs = 0;
  if (segment.bold) attrs |= TextAttributes.BOLD;
  if (segment.dim) attrs |= TextAttributes.DIM;
  if (segment.italic) attrs |= TextAttributes.ITALIC;
  if (segment.underline) attrs |= TextAttributes.UNDERLINE;
  if (segment.blink) attrs |= TextAttributes.BLINK;
  if (segment.inverse) attrs |= TextAttributes.INVERSE;
  if (segment.strikethrough) attrs |= TextAttributes.STRIKETHROUGH;
  return attrs;
}
