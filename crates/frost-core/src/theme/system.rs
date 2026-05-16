use crate::theme::types::{RGBA, TerminalColors, ThemeJson, ThemeMode, ThemeValue};
use std::collections::HashMap;

fn luminance(rgba: RGBA) -> f32 {
    let [r, g, b] = [rgba.r, rgba.g, rgba.b];
    let linearize = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
}

fn blend_over(a: RGBA, b: RGBA) -> RGBA {
    let alpha = a.a + b.a * (1.0 - a.a);
    if alpha == 0.0 {
        return RGBA::TRANSPARENT;
    }
    RGBA {
        r: (a.r * a.a + b.r * b.a * (1.0 - a.a)) / alpha,
        g: (a.g * a.a + b.g * b.a * (1.0 - a.a)) / alpha,
        b: (a.b * a.a + b.b * b.a * (1.0 - a.a)) / alpha,
        a: alpha,
    }
}

fn grayscale_ramp(bg_luminance: f32, steps: usize) -> Vec<String> {
    let mut ramp = Vec::new();
    let is_dark = bg_luminance <= 0.5;

    for i in 1..=steps {
        let t = i as f32 / (steps as f32 + 1.0);
        let gray = if is_dark {
            (10.0 + t * 200.0).round()
        } else {
            (245.0 - t * 200.0).round()
        };
        let gray = gray.max(0.0).min(255.0) as u8;
        ramp.push(format!("#{:02x}{:02x}{:02x}", gray, gray, gray));
    }
    ramp
}

fn ramp_index(steps: usize, idx: usize) -> usize {
    idx.max(0).min(steps - 1)
}

fn rgba_to_string(rgba: RGBA) -> String {
    rgba.to_hex()
}

fn semantic_from_ansi(palette: &TerminalColors, idx: usize) -> String {
    if idx < palette.palette.len() {
        rgba_to_string(palette.palette[idx])
    } else {
        rgba_to_string(crate::theme::resolver::ansi_to_rgba(idx as u8))
    }
}

pub fn generate_system_theme(palette: &TerminalColors, mode: ThemeMode) -> ThemeJson {
    let bg = palette.background;
    let fg = palette.foreground;
    let bg_lum = luminance(bg);
    let is_dark = bg_lum <= 0.5 || mode == ThemeMode::Dark;

    let ramp = grayscale_ramp(bg_lum, 12);

    let defs: HashMap<String, crate::theme::types::ThemeDefValue> = HashMap::new();

    let ramp_str = |idx: usize| -> String { ramp[ramp_index(12, idx)].clone() };

    let muted_text_lum = if is_dark {
        bg_lum + 0.35
    } else {
        bg_lum - 0.35
    };
    let muted_text_gray = (muted_text_lum.min(1.0).max(0.0) * 255.0).round() as u8;

    let diff_alpha = if is_dark { 0.22 } else { 0.14 };
    let diff_added_bg = blend_over(
        RGBA {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: diff_alpha,
        },
        bg,
    );
    let diff_removed_bg = blend_over(
        RGBA {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: diff_alpha,
        },
        bg,
    );

    let mut theme = HashMap::new();
    theme.insert(
        "primary".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 4)),
    );
    theme.insert(
        "secondary".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 5)),
    );
    theme.insert(
        "accent".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 6)),
    );
    theme.insert(
        "error".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 1)),
    );
    theme.insert(
        "warning".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 3)),
    );
    theme.insert(
        "success".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 2)),
    );
    theme.insert(
        "info".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 6)),
    );
    theme.insert("text".to_string(), ThemeValue::String(rgba_to_string(fg)));
    theme.insert(
        "textMuted".to_string(),
        ThemeValue::String(format!(
            "#{:02x}{:02x}{:02x}",
            muted_text_gray, muted_text_gray, muted_text_gray
        )),
    );
    theme.insert(
        "selectedListItemText".to_string(),
        ThemeValue::String(if is_dark { ramp_str(0) } else { ramp_str(11) }),
    );
    theme.insert(
        "background".to_string(),
        ThemeValue::String("transparent".to_string()),
    );
    theme.insert(
        "backgroundPanel".to_string(),
        ThemeValue::String(ramp_str(if is_dark { 1 } else { 10 })),
    );
    theme.insert(
        "backgroundElement".to_string(),
        ThemeValue::String(ramp_str(if is_dark { 2 } else { 9 })),
    );
    theme.insert(
        "backgroundMenu".to_string(),
        ThemeValue::String(ramp_str(if is_dark { 2 } else { 9 })),
    );
    theme.insert(
        "border".to_string(),
        ThemeValue::String(ramp_str(if is_dark { 4 } else { 7 })),
    );
    theme.insert(
        "borderActive".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 4)),
    );
    theme.insert(
        "borderSubtle".to_string(),
        ThemeValue::String(ramp_str(if is_dark { 3 } else { 8 })),
    );
    theme.insert(
        "diffAdded".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 2)),
    );
    theme.insert(
        "diffRemoved".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 1)),
    );
    theme.insert(
        "diffContext".to_string(),
        ThemeValue::String(rgba_to_string(fg)),
    );
    theme.insert(
        "diffHunkHeader".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 4)),
    );
    theme.insert(
        "diffHighlightAdded".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 2)),
    );
    theme.insert(
        "diffHighlightRemoved".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 1)),
    );
    theme.insert(
        "diffAddedBg".to_string(),
        ThemeValue::String(rgba_to_string(diff_added_bg)),
    );
    theme.insert(
        "diffRemovedBg".to_string(),
        ThemeValue::String(rgba_to_string(diff_removed_bg)),
    );
    theme.insert(
        "diffContextBg".to_string(),
        ThemeValue::String(rgba_to_string(bg)),
    );
    theme.insert(
        "diffLineNumber".to_string(),
        ThemeValue::String(ramp_str(if is_dark { 5 } else { 6 })),
    );
    theme.insert(
        "diffAddedLineNumberBg".to_string(),
        ThemeValue::String(rgba_to_string(diff_added_bg)),
    );
    theme.insert(
        "diffRemovedLineNumberBg".to_string(),
        ThemeValue::String(rgba_to_string(diff_removed_bg)),
    );
    theme.insert(
        "markdownText".to_string(),
        ThemeValue::String(rgba_to_string(fg)),
    );
    theme.insert(
        "markdownHeading".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 4)),
    );
    theme.insert(
        "markdownLink".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 6)),
    );
    theme.insert(
        "markdownLinkText".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 6)),
    );
    theme.insert(
        "markdownCode".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 3)),
    );
    theme.insert(
        "markdownBlockQuote".to_string(),
        ThemeValue::String(ramp_str(if is_dark { 5 } else { 6 })),
    );
    theme.insert(
        "markdownEmph".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 5)),
    );
    theme.insert(
        "markdownStrong".to_string(),
        ThemeValue::String(rgba_to_string(fg)),
    );
    theme.insert(
        "markdownHorizontalRule".to_string(),
        ThemeValue::String(ramp_str(if is_dark { 4 } else { 7 })),
    );
    theme.insert(
        "markdownListItem".to_string(),
        ThemeValue::String(rgba_to_string(fg)),
    );
    theme.insert(
        "markdownListEnumeration".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 5)),
    );
    theme.insert(
        "markdownImage".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 6)),
    );
    theme.insert(
        "markdownImageText".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 6)),
    );
    theme.insert(
        "markdownCodeBlock".to_string(),
        ThemeValue::String(rgba_to_string(bg)),
    );
    theme.insert(
        "syntaxComment".to_string(),
        ThemeValue::String(ramp_str(if is_dark { 5 } else { 6 })),
    );
    theme.insert(
        "syntaxKeyword".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 5)),
    );
    theme.insert(
        "syntaxFunction".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 4)),
    );
    theme.insert(
        "syntaxVariable".to_string(),
        ThemeValue::String(rgba_to_string(fg)),
    );
    theme.insert(
        "syntaxString".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 2)),
    );
    theme.insert(
        "syntaxNumber".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 3)),
    );
    theme.insert(
        "syntaxType".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 3)),
    );
    theme.insert(
        "syntaxOperator".to_string(),
        ThemeValue::String(semantic_from_ansi(palette, 6)),
    );
    theme.insert(
        "syntaxPunctuation".to_string(),
        ThemeValue::String(ramp_str(if is_dark { 5 } else { 6 })),
    );
    theme.insert("thinkingOpacity".to_string(), ThemeValue::Number(0.6));

    ThemeJson {
        schema: None,
        defs: Some(defs),
        theme,
    }
}
