import { describe, it, expect } from "bun:test";
import { generateSyntax, generateSubtleSyntax } from "@/tui/theme/syntax";
import { resolveTheme } from "@/tui/theme/resolver";
import { DEFAULT_THEMES } from "@/tui/theme/builtin";

describe("Syntax generators", () => {
  const resolved = resolveTheme(DEFAULT_THEMES.opencode!, "dark");

  it("generateSyntax produces all style fields", () => {
    const s = generateSyntax(resolved);
    expect(s.comment).toBeDefined();
    expect(s.keyword).toBeDefined();
    expect(s.function).toBeDefined();
    expect(s.variable).toBeDefined();
    expect(s.string).toBeDefined();
    expect(s.number).toBeDefined();
    expect(s.type).toBeDefined();
    expect(s.operator).toBeDefined();
    expect(s.punctuation).toBeDefined();
  });

  it("generateSubtleSyntax applies thinking opacity", () => {
    const subtle = generateSubtleSyntax(resolved);

    expect(subtle.comment).toContain("rgba(");
    expect(subtle.comment).toContain(String(resolved.thinkingOpacity));
  });

  it("both generators return same structure", () => {
    const normal = generateSyntax(resolved);
    const subtle = generateSubtleSyntax(resolved);
    expect(Object.keys(normal)).toEqual(Object.keys(subtle));
  });
});
