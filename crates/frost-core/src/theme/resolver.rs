use crate::theme::types::{ResolvedTheme, RGBA, ThemeDefValue, ThemeJson, ThemeMode, ThemeValue};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Theme resolve error: {0}")]
pub struct ResolveError(pub String);

const HEX_RE: &str = r"^#([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})?$";
const SHORT_HEX_RE: &str = r"^#([0-9a-fA-F])([0-9a-fA-F])([0-9a-fA-F])([0-9a-fA-F])?$";

pub fn parse_hex(hex: &str) -> Result<RGBA, ResolveError> {
    let short_re = regex_lite::Regex::new(SHORT_HEX_RE).unwrap();
    if let Some(caps) = short_re.captures(hex) {
        let r = u8::from_str_radix(&format!("{}{}", &caps[1], &caps[1]), 16).unwrap();
        let g = u8::from_str_radix(&format!("{}{}", &caps[2], &caps[2]), 16).unwrap();
        let b = u8::from_str_radix(&format!("{}{}", &caps[3], &caps[3]), 16).unwrap();
        let a = if let Some(a_cap) = caps.get(4) {
            u8::from_str_radix(&format!("{}{}", a_cap.as_str(), a_cap.as_str()), 16).unwrap() as f32
                / 255.0
        } else {
            1.0
        };
        return Ok(RGBA::from_u8(r, g, b, a));
    }

    let long_re = regex_lite::Regex::new(HEX_RE).unwrap();
    let caps = long_re
        .captures(hex)
        .ok_or_else(|| ResolveError(format!("Invalid hex color: {}", hex)))?;

    let r = u8::from_str_radix(&caps[1], 16).unwrap();
    let g = u8::from_str_radix(&caps[2], 16).unwrap();
    let b = u8::from_str_radix(&caps[3], 16).unwrap();
    let a = if let Some(a_cap) = caps.get(4) {
        u8::from_str_radix(a_cap.as_str(), 16).unwrap() as f32 / 255.0
    } else {
        1.0
    };

    Ok(RGBA::from_u8(r, g, b, a))
}

fn parse_css_rgba(str: &str) -> Result<RGBA, ResolveError> {
    let inner = str
        .trim_start_matches("rgba(")
        .trim_start_matches("rgb(")
        .trim_end_matches(')');
    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return Err(ResolveError(format!("Invalid rgb/rgba: {}", str)));
    }
    let r = parts[0].parse::<u8>().map_err(|e| ResolveError(e.to_string()))?;
    let g = parts[1].parse::<u8>().map_err(|e| ResolveError(e.to_string()))?;
    let b = parts[2].parse::<u8>().map_err(|e| ResolveError(e.to_string()))?;
    let a = parts.get(3).and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.0);
    Ok(RGBA::from_u8(r, g, b, a))
}

pub fn ansi_to_rgba(index: u8) -> RGBA {
    let ansi16: [RGBA; 16] = [
        RGBA::from_u8(0, 0, 0, 1.0),
        RGBA::from_u8(128, 0, 0, 1.0),
        RGBA::from_u8(0, 128, 0, 1.0),
        RGBA::from_u8(128, 128, 0, 1.0),
        RGBA::from_u8(0, 0, 128, 1.0),
        RGBA::from_u8(128, 0, 128, 1.0),
        RGBA::from_u8(0, 128, 128, 1.0),
        RGBA::from_u8(192, 192, 192, 1.0),
        RGBA::from_u8(128, 128, 128, 1.0),
        RGBA::from_u8(255, 0, 0, 1.0),
        RGBA::from_u8(0, 255, 0, 1.0),
        RGBA::from_u8(255, 255, 0, 1.0),
        RGBA::from_u8(0, 0, 255, 1.0),
        RGBA::from_u8(255, 0, 255, 1.0),
        RGBA::from_u8(0, 255, 255, 1.0),
        RGBA::from_u8(255, 255, 255, 1.0),
    ];

    if index < 16 {
        return ansi16[index as usize];
    }

    if index < 232 {
        let i = index - 16;
        let r = (i / 36) % 6;
        let g = (i / 6) % 6;
        let b = i % 6;
        return RGBA::from_u8(
            ((r as f32 / 5.0) * 255.0).round() as u8,
            ((g as f32 / 5.0) * 255.0).round() as u8,
            ((b as f32 / 5.0) * 255.0).round() as u8,
            1.0,
        );
    }

    let gray = ((index - 232) as f32 / 23.0 * 255.0).round() as u8;
    RGBA::from_u8(gray, gray, gray, 1.0)
}

pub fn resolve_color(
    value: &ThemeValue,
    defs: &std::collections::HashMap<String, ThemeDefValue>,
    theme: &std::collections::HashMap<String, ThemeValue>,
    mode: ThemeMode,
    chain: &mut Vec<String>,
) -> Result<RGBA, ResolveError> {
    match value {
        ThemeValue::Number(n) => Ok(ansi_to_rgba(*n as u8)),
        ThemeValue::Variant { dark, light } => {
            let s = match mode {
                ThemeMode::Dark => dark,
                ThemeMode::Light => light,
            };
            resolve_color(&ThemeValue::String(s.clone()), defs, theme, mode, chain)
        }
        ThemeValue::String(str) => {
            if str == "transparent" || str == "none" {
                return Ok(RGBA::TRANSPARENT);
            }
            if str.starts_with('#') {
                return parse_hex(str);
            }
            if str.starts_with("rgba(") || str.starts_with("rgb(") {
                return parse_css_rgba(str);
            }

            // Reference resolution
            if chain.contains(str) {
                return Err(ResolveError(format!(
                    "Circular reference detected: {} -> {}",
                    chain.join(" -> "),
                    str
                )));
            }

            // Try defs first
            if let Some(def_value) = defs.get(str) {
                let def_str = def_value.as_string();
                if def_str.starts_with('#') {
                    return parse_hex(&def_str);
                }
                if def_str.starts_with("rgba(") || def_str.starts_with("rgb(") {
                    return parse_css_rgba(&def_str);
                }
                chain.push(str.clone());
                let result = resolve_color(
                    &ThemeValue::String(def_str),
                    defs,
                    theme,
                    mode,
                    chain,
                );
                chain.pop();
                return result;
            }

            // Try theme tokens
            if let Some(theme_value) = theme.get(str) {
                chain.push(str.clone());
                let result = resolve_color(theme_value, defs, theme, mode, chain);
                chain.pop();
                return result;
            }

            Err(ResolveError(format!("Missing reference: {}", str)))
        }
    }
}

fn resolve_optional(
    key: &str,
    defs: &std::collections::HashMap<String, ThemeDefValue>,
    theme: &std::collections::HashMap<String, ThemeValue>,
    mode: ThemeMode,
) -> Option<RGBA> {
    let value = theme.get(key)?;
    let mut chain = Vec::new();
    resolve_color(value, defs, theme, mode, &mut chain).ok()
}

pub fn resolve_theme(theme_json: &ThemeJson, mode: ThemeMode) -> Result<ResolvedTheme, ResolveError> {
    let defs = theme_json.defs.clone().unwrap_or_default();
    let theme = &theme_json.theme;

    let r = |key: &str| -> Result<RGBA, ResolveError> {
        let value = theme.get(key).ok_or_else(|| {
            ResolveError(format!("Missing theme token: {}", key))
        })?;
        let mut chain = Vec::new();
        resolve_color(value, &defs, theme, mode, &mut chain)
    };

    Ok(ResolvedTheme {
        primary: r("primary")?,
        secondary: r("secondary")?,
        accent: r("accent")?,
        error: r("error")?,
        warning: r("warning")?,
        success: r("success")?,
        info: r("info")?,
        text: r("text")?,
        text_muted: r("textMuted")?,
        selected_list_item_text: resolve_optional("selectedListItemText", &defs, theme, mode)
            .unwrap_or_else(|| r("background").unwrap_or(RGBA::TRANSPARENT)),
        background: r("background")?,
        background_panel: r("backgroundPanel")?,
        background_element: r("backgroundElement")?,
        background_menu: resolve_optional("backgroundMenu", &defs, theme, mode)
            .unwrap_or_else(|| r("backgroundElement").unwrap_or(RGBA::TRANSPARENT)),
        border: r("border")?,
        border_active: r("borderActive")?,
        border_subtle: r("borderSubtle")?,
        diff_added: r("diffAdded")?,
        diff_removed: r("diffRemoved")?,
        diff_context: r("diffContext")?,
        diff_hunk_header: r("diffHunkHeader")?,
        diff_highlight_added: r("diffHighlightAdded")?,
        diff_highlight_removed: r("diffHighlightRemoved")?,
        diff_added_bg: r("diffAddedBg")?,
        diff_removed_bg: r("diffRemovedBg")?,
        diff_context_bg: r("diffContextBg")?,
        diff_line_number: r("diffLineNumber")?,
        diff_added_line_number_bg: r("diffAddedLineNumberBg")?,
        diff_removed_line_number_bg: r("diffRemovedLineNumberBg")?,
        markdown_text: r("markdownText")?,
        markdown_heading: r("markdownHeading")?,
        markdown_link: r("markdownLink")?,
        markdown_link_text: r("markdownLinkText")?,
        markdown_code: r("markdownCode")?,
        markdown_block_quote: r("markdownBlockQuote")?,
        markdown_emph: r("markdownEmph")?,
        markdown_strong: r("markdownStrong")?,
        markdown_horizontal_rule: r("markdownHorizontalRule")?,
        markdown_list_item: r("markdownListItem")?,
        markdown_list_enumeration: r("markdownListEnumeration")?,
        markdown_image: r("markdownImage")?,
        markdown_image_text: r("markdownImageText")?,
        markdown_code_block: r("markdownCodeBlock")?,
        syntax_comment: r("syntaxComment")?,
        syntax_keyword: r("syntaxKeyword")?,
        syntax_function: r("syntaxFunction")?,
        syntax_variable: r("syntaxVariable")?,
        syntax_string: r("syntaxString")?,
        syntax_number: r("syntaxNumber")?,
        syntax_type: r("syntaxType")?,
        syntax_operator: r("syntaxOperator")?,
        syntax_punctuation: r("syntaxPunctuation")?,
        thinking_opacity: theme
            .get("thinkingOpacity")
            .and_then(|v| match v {
                ThemeValue::Number(n) => Some(*n),
                _ => None,
            })
            .unwrap_or(0.6),
    })
}

pub fn resolve_theme_safe(theme_json: &ThemeJson, mode: ThemeMode) -> Option<ResolvedTheme> {
    resolve_theme(theme_json, mode).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_hex("#ff0000").unwrap(), RGBA::from_u8(255, 0, 0, 1.0));
        assert_eq!(parse_hex("#00ff00").unwrap(), RGBA::from_u8(0, 255, 0, 1.0));
        assert_eq!(parse_hex("#0000ff").unwrap(), RGBA::from_u8(0, 0, 255, 1.0));
        assert_eq!(parse_hex("#fff").unwrap(), RGBA::from_u8(255, 255, 255, 1.0));
        assert_eq!(parse_hex("#f00").unwrap(), RGBA::from_u8(255, 0, 0, 1.0));
        assert_eq!(parse_hex("#ff000080").unwrap(), RGBA::from_u8(255, 0, 0, 128.0 / 255.0));
    }

    #[test]
    fn test_ansi_to_rgba() {
        assert_eq!(ansi_to_rgba(0), RGBA::from_u8(0, 0, 0, 1.0));
        assert_eq!(ansi_to_rgba(15), RGBA::from_u8(255, 255, 255, 1.0));
    }

    #[test]
    fn test_resolve_color_basic() {
        let defs = HashMap::new();
        let theme = HashMap::new();
        let mut chain = Vec::new();

        let value = ThemeValue::String("#ff0000".to_string());
        assert_eq!(
            resolve_color(&value, &defs, &theme, ThemeMode::Dark, &mut chain).unwrap(),
            RGBA::from_u8(255, 0, 0, 1.0)
        );
    }

    #[test]
    fn test_resolve_color_variant() {
        let defs = HashMap::new();
        let theme = HashMap::new();
        let mut chain = Vec::new();

        let value = ThemeValue::Variant {
            dark: "#000000".to_string(),
            light: "#ffffff".to_string(),
        };
        assert_eq!(
            resolve_color(&value, &defs, &theme, ThemeMode::Dark, &mut chain).unwrap(),
            RGBA::from_u8(0, 0, 0, 1.0)
        );
        assert_eq!(
            resolve_color(&value, &defs, &theme, ThemeMode::Light, &mut chain).unwrap(),
            RGBA::from_u8(255, 255, 255, 1.0)
        );
    }

    #[test]
    fn test_resolve_color_reference() {
        let mut defs = HashMap::new();
        defs.insert("myRed".to_string(), ThemeDefValue::String("#ff0000".to_string()));
        let theme = HashMap::new();
        let mut chain = Vec::new();

        let value = ThemeValue::String("myRed".to_string());
        assert_eq!(
            resolve_color(&value, &defs, &theme, ThemeMode::Dark, &mut chain).unwrap(),
            RGBA::from_u8(255, 0, 0, 1.0)
        );
    }

    #[test]
    fn test_resolve_theme_opencode() {
        let theme_json = crate::theme::builtin::builtin_themes()
            .remove("opencode")
            .unwrap();
        let resolved = resolve_theme(&theme_json, ThemeMode::Dark).unwrap();
        assert!(resolved.primary.r > 0.0);
    }
}
