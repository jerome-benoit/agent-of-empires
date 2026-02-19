use anyhow::{bail, Result};
use ratatui::style::Color;

/// Parse a hex color string to ratatui Color.
///
/// Supports two formats:
/// - `#RRGGBB` (6 hex digits) - e.g., "#39ff14" -> Rgb(57, 255, 20)
/// - `#RGB` (3 hex digits shorthand) - e.g., "#fff" -> Rgb(255, 255, 255)
///
/// # Examples
///
/// ```
/// use agent_of_empires::tui::themes::color::parse_hex_color;
/// use ratatui::style::Color;
///
/// assert_eq!(parse_hex_color("#ffffff").unwrap(), Color::Rgb(255, 255, 255));
/// assert_eq!(parse_hex_color("#fff").unwrap(), Color::Rgb(255, 255, 255));
/// ```
///
/// # Errors
///
/// Returns error if:
/// - String doesn't start with `#`
/// - Hex string is not 3 or 6 characters
/// - Contains non-hexadecimal characters
pub fn parse_hex_color(s: &str) -> Result<Color> {
    let hex = s
        .strip_prefix('#')
        .ok_or_else(|| anyhow::anyhow!("Color must start with '#', got: {}", s))?;

    match hex.len() {
        3 => {
            let r = expand_hex_digit(hex.chars().next().unwrap())?;
            let g = expand_hex_digit(hex.chars().nth(1).unwrap())?;
            let b = expand_hex_digit(hex.chars().nth(2).unwrap())?;
            Ok(Color::Rgb(r, g, b))
        }
        6 => {
            let r = hex_pair_to_u8(&hex[0..2])?;
            let g = hex_pair_to_u8(&hex[2..4])?;
            let b = hex_pair_to_u8(&hex[4..6])?;
            Ok(Color::Rgb(r, g, b))
        }
        len => bail!("Hex color must be 3 or 6 characters (got {}): {}", len, s),
    }
}

/// Expand a single hex digit to a pair (e.g., 'F' -> "FF" -> 255)
fn expand_hex_digit(c: char) -> Result<u8> {
    let hex_char = format!("{}{}", c, c);
    hex_pair_to_u8(&hex_char)
}

/// Convert a pair of hex digits to u8 (e.g., "FF" -> 255)
fn hex_pair_to_u8(hex: &str) -> Result<u8> {
    u8::from_str_radix(hex, 16).map_err(|_| {
        anyhow::anyhow!(
            "Invalid hex color component: {} (must be valid hex digits)",
            hex
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_color_parsing_valid_six_digit() {
        assert_eq!(parse_hex_color("#000000").unwrap(), Color::Rgb(0, 0, 0));
        assert_eq!(
            parse_hex_color("#ffffff").unwrap(),
            Color::Rgb(255, 255, 255)
        );
        assert_eq!(parse_hex_color("#39ff14").unwrap(), Color::Rgb(57, 255, 20));
        assert_eq!(parse_hex_color("#ff0000").unwrap(), Color::Rgb(255, 0, 0));
        assert_eq!(parse_hex_color("#00ff00").unwrap(), Color::Rgb(0, 255, 0));
        assert_eq!(parse_hex_color("#0000ff").unwrap(), Color::Rgb(0, 0, 255));
    }

    #[test]
    fn test_hex_color_parsing_valid_three_digit() {
        assert_eq!(parse_hex_color("#fff").unwrap(), Color::Rgb(255, 255, 255));
        assert_eq!(parse_hex_color("#000").unwrap(), Color::Rgb(0, 0, 0));
        assert_eq!(parse_hex_color("#abc").unwrap(), Color::Rgb(170, 187, 204));
        assert_eq!(parse_hex_color("#f00").unwrap(), Color::Rgb(255, 0, 0));
        assert_eq!(parse_hex_color("#0f0").unwrap(), Color::Rgb(0, 255, 0));
        assert_eq!(parse_hex_color("#00f").unwrap(), Color::Rgb(0, 0, 255));
    }

    #[test]
    fn test_hex_color_parsing_case_insensitive() {
        assert_eq!(
            parse_hex_color("#FFFFFF").unwrap(),
            Color::Rgb(255, 255, 255)
        );
        assert_eq!(
            parse_hex_color("#FfFfFf").unwrap(),
            Color::Rgb(255, 255, 255)
        );
        assert_eq!(parse_hex_color("#ABC").unwrap(), Color::Rgb(170, 187, 204));
    }

    #[test]
    fn test_hex_color_errors_missing_hash() {
        let result = parse_hex_color("ffffff");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must start with '#'"));
    }

    #[test]
    fn test_hex_color_errors_invalid_chars() {
        assert!(parse_hex_color("#gg0000").is_err());
        assert!(parse_hex_color("#zzz").is_err());
        assert!(parse_hex_color("#12345g").is_err());
    }

    #[test]
    fn test_hex_color_errors_invalid_length() {
        assert!(parse_hex_color("#").is_err());
        assert!(parse_hex_color("#ff").is_err());
        assert!(parse_hex_color("#1234567").is_err());
        assert!(parse_hex_color("").is_err());
    }

    #[test]
    fn test_hex_color_errors_empty_string() {
        let result = parse_hex_color("");
        assert!(result.is_err());
    }
}
