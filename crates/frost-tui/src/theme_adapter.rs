use frost_core::RGBA;
use ratatui::style::Color;

/// Convert an RGBA color to a ratatui Color.
pub fn to_color(rgba: RGBA) -> Color {
    Color::Rgb(
        (rgba.r * 255.0) as u8,
        (rgba.g * 255.0) as u8,
        (rgba.b * 255.0) as u8,
    )
}
