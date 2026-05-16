use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeJson {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defs: Option<HashMap<String, ThemeDefValue>>,

    pub theme: HashMap<String, ThemeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeValue {
    String(String),
    Number(f64),
    Variant { dark: String, light: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeDefValue {
    String(String),
    Number(f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RGBA {
    pub r: f32, // 0.0–1.0
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    pub primary: RGBA,
    pub secondary: RGBA,
    pub accent: RGBA,
    pub error: RGBA,
    pub warning: RGBA,
    pub success: RGBA,
    pub info: RGBA,
    pub text: RGBA,
    pub text_muted: RGBA,
    pub selected_list_item_text: RGBA,
    pub background: RGBA,
    pub background_panel: RGBA,
    pub background_element: RGBA,
    pub background_menu: RGBA,
    pub border: RGBA,
    pub border_active: RGBA,
    pub border_subtle: RGBA,
    pub diff_added: RGBA,
    pub diff_removed: RGBA,
    pub diff_context: RGBA,
    pub diff_hunk_header: RGBA,
    pub diff_highlight_added: RGBA,
    pub diff_highlight_removed: RGBA,
    pub diff_added_bg: RGBA,
    pub diff_removed_bg: RGBA,
    pub diff_context_bg: RGBA,
    pub diff_line_number: RGBA,
    pub diff_added_line_number_bg: RGBA,
    pub diff_removed_line_number_bg: RGBA,
    pub markdown_text: RGBA,
    pub markdown_heading: RGBA,
    pub markdown_link: RGBA,
    pub markdown_link_text: RGBA,
    pub markdown_code: RGBA,
    pub markdown_block_quote: RGBA,
    pub markdown_emph: RGBA,
    pub markdown_strong: RGBA,
    pub markdown_horizontal_rule: RGBA,
    pub markdown_list_item: RGBA,
    pub markdown_list_enumeration: RGBA,
    pub markdown_image: RGBA,
    pub markdown_image_text: RGBA,
    pub markdown_code_block: RGBA,
    pub syntax_comment: RGBA,
    pub syntax_keyword: RGBA,
    pub syntax_function: RGBA,
    pub syntax_variable: RGBA,
    pub syntax_string: RGBA,
    pub syntax_number: RGBA,
    pub syntax_type: RGBA,
    pub syntax_operator: RGBA,
    pub syntax_punctuation: RGBA,
    pub thinking_opacity: f64,
}

#[derive(Debug, Clone)]
pub struct TerminalColors {
    pub foreground: RGBA,
    pub background: RGBA,
    pub palette: Vec<RGBA>,
}

impl RGBA {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const TRANSPARENT: RGBA = RGBA {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub fn from_u8(r: u8, g: u8, b: u8, a: f32) -> Self {
        RGBA {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a,
        }
    }

    pub fn to_hex(&self) -> String {
        if self.a >= 1.0 {
            format!(
                "#{:02x}{:02x}{:02x}",
                (self.r * 255.0).round() as u8,
                (self.g * 255.0).round() as u8,
                (self.b * 255.0).round() as u8,
            )
        } else {
            format!(
                "rgba({},{},{},{})",
                (self.r * 255.0).round() as u8,
                (self.g * 255.0).round() as u8,
                (self.b * 255.0).round() as u8,
                self.a
            )
        }
    }
}

impl ThemeDefValue {
    pub fn as_string(&self) -> String {
        match self {
            ThemeDefValue::String(s) => s.clone(),
            ThemeDefValue::Number(n) => n.to_string(),
        }
    }
}

impl ThemeValue {
    pub fn as_string(&self) -> String {
        match self {
            ThemeValue::String(s) => s.clone(),
            ThemeValue::Number(n) => n.to_string(),
            ThemeValue::Variant { dark, .. } => dark.clone(),
        }
    }
}
