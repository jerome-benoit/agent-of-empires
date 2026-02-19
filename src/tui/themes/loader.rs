use crate::tui::styles::Theme;
use tracing::warn;

pub const AVAILABLE_THEMES: &[&str] = &["phosphor", "tokyo-night", "catppuccin-latte"];

const PHOSPHOR_TOML: &str = include_str!("phosphor.toml");
const TOKYO_NIGHT_TOML: &str = include_str!("tokyo-night.toml");
const CATPPUCCIN_LATTE_TOML: &str = include_str!("catppuccin-latte.toml");

pub fn load_theme(name: &str) -> Theme {
    let toml_str = match name {
        "phosphor" => PHOSPHOR_TOML,
        "tokyo-night" => TOKYO_NIGHT_TOML,
        "catppuccin-latte" => CATPPUCCIN_LATTE_TOML,
        _ => {
            warn!("Unknown theme '{}', falling back to phosphor", name);
            PHOSPHOR_TOML
        }
    };

    match toml::from_str(toml_str) {
        Ok(theme) => theme,
        Err(e) => {
            warn!(
                "Failed to parse theme '{}': {}, using default phosphor",
                name, e
            );
            Theme::phosphor()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_load_phosphor() {
        let theme = load_theme("phosphor");
        assert_eq!(*theme.title, Color::Rgb(57, 255, 20));
        assert_eq!(*theme.background, Color::Rgb(16, 20, 18));
    }

    #[test]
    fn test_load_tokyo_night() {
        let theme = load_theme("tokyo-night");
        assert_eq!(*theme.title, Color::Rgb(122, 162, 247));
        assert_eq!(*theme.background, Color::Rgb(26, 27, 38));
    }

    #[test]
    fn test_load_catppuccin_latte() {
        let theme = load_theme("catppuccin-latte");
        assert_eq!(*theme.title, Color::Rgb(30, 102, 245));
        assert_eq!(*theme.background, Color::Rgb(239, 241, 245));
    }

    #[test]
    fn test_load_invalid_fallback() {
        let theme = load_theme("nonexistent-theme");
        assert_eq!(*theme.title, Color::Rgb(57, 255, 20));
        assert_eq!(*theme.background, Color::Rgb(16, 20, 18));
    }

    #[test]
    fn test_available_themes_count() {
        assert_eq!(AVAILABLE_THEMES.len(), 3);
        assert!(AVAILABLE_THEMES.contains(&"phosphor"));
        assert!(AVAILABLE_THEMES.contains(&"tokyo-night"));
        assert!(AVAILABLE_THEMES.contains(&"catppuccin-latte"));
    }
}
