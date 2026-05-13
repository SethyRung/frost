import type { ResolvedTheme, SyntaxStyle } from "./types";
import { rgbaToString } from "./resolver";

export function generateSyntax(theme: ResolvedTheme): SyntaxStyle {
  return {
    comment: rgbaToString(theme.syntaxComment),
    keyword: rgbaToString(theme.syntaxKeyword),
    function: rgbaToString(theme.syntaxFunction),
    variable: rgbaToString(theme.syntaxVariable),
    string: rgbaToString(theme.syntaxString),
    number: rgbaToString(theme.syntaxNumber),
    type: rgbaToString(theme.syntaxType),
    operator: rgbaToString(theme.syntaxOperator),
    punctuation: rgbaToString(theme.syntaxPunctuation),
  };
}

export function generateSubtleSyntax(theme: ResolvedTheme): SyntaxStyle {
  const opacity = theme.thinkingOpacity;
  const applyOpacity = (rgba: { r: number; g: number; b: number }): string => {
    return `rgba(${rgba.r},${rgba.g},${rgba.b},${opacity})`;
  };

  return {
    comment: applyOpacity(theme.syntaxComment),
    keyword: applyOpacity(theme.syntaxKeyword),
    function: applyOpacity(theme.syntaxFunction),
    variable: applyOpacity(theme.syntaxVariable),
    string: applyOpacity(theme.syntaxString),
    number: applyOpacity(theme.syntaxNumber),
    type: applyOpacity(theme.syntaxType),
    operator: applyOpacity(theme.syntaxOperator),
    punctuation: applyOpacity(theme.syntaxPunctuation),
  };
}
