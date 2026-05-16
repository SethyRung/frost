use crate::theme::types::ThemeJson;
use std::collections::HashMap;

pub const DEFAULT_THEME_ID: &str = "opencode";

macro_rules! load_theme {
    ($themes:expr, $name:expr, $path:expr) => {
        {
            let json: ThemeJson = serde_json::from_str(include_str!($path))
                .expect(concat!("Failed to parse built-in theme: ", $name));
            $themes.insert($name.to_string(), json);
        }
    };
}

pub fn builtin_themes() -> HashMap<String, ThemeJson> {
    let mut themes = HashMap::new();

    load_theme!(themes, "opencode", "../../../../themes/opencode.json");
    load_theme!(themes, "dracula", "../../../../themes/dracula.json");
    load_theme!(themes, "aura", "../../../../themes/aura.json");
    load_theme!(themes, "ayu", "../../../../themes/ayu.json");
    load_theme!(themes, "carbonfox", "../../../../themes/carbonfox.json");
    load_theme!(themes, "catppuccin", "../../../../themes/catppuccin.json");
    load_theme!(themes, "catppuccin-frappe", "../../../../themes/catppuccin-frappe.json");
    load_theme!(themes, "catppuccin-macchiato", "../../../../themes/catppuccin-macchiato.json");
    load_theme!(themes, "cobalt2", "../../../../themes/cobalt2.json");
    load_theme!(themes, "cursor", "../../../../themes/cursor.json");
    load_theme!(themes, "everforest", "../../../../themes/everforest.json");
    load_theme!(themes, "flexoki", "../../../../themes/flexoki.json");
    load_theme!(themes, "github", "../../../../themes/github.json");
    load_theme!(themes, "gruvbox", "../../../../themes/gruvbox.json");
    load_theme!(themes, "kanagawa", "../../../../themes/kanagawa.json");
    load_theme!(themes, "lucent-orng", "../../../../themes/lucent-orng.json");
    load_theme!(themes, "material", "../../../../themes/material.json");
    load_theme!(themes, "matrix", "../../../../themes/matrix.json");
    load_theme!(themes, "mercury", "../../../../themes/mercury.json");
    load_theme!(themes, "monokai", "../../../../themes/monokai.json");
    load_theme!(themes, "nightowl", "../../../../themes/nightowl.json");
    load_theme!(themes, "nord", "../../../../themes/nord.json");
    load_theme!(themes, "one-dark", "../../../../themes/one-dark.json");
    load_theme!(themes, "orng", "../../../../themes/orng.json");
    load_theme!(themes, "osaka-jade", "../../../../themes/osaka-jade.json");
    load_theme!(themes, "palenight", "../../../../themes/palenight.json");
    load_theme!(themes, "rosepine", "../../../../themes/rosepine.json");
    load_theme!(themes, "solarized", "../../../../themes/solarized.json");
    load_theme!(themes, "synthwave84", "../../../../themes/synthwave84.json");
    load_theme!(themes, "tokyonight", "../../../../themes/tokyonight.json");
    load_theme!(themes, "vercel", "../../../../themes/vercel.json");
    load_theme!(themes, "vesper", "../../../../themes/vesper.json");
    load_theme!(themes, "zenburn", "../../../../themes/zenburn.json");

    themes
}
