//! Theme model for GitCactus.
//!
//! GitCactus stays intentionally restrained by default: mostly
//! grayscale with a few subtle accents. This module exposes a small
//! set of **color roles** that screens can pull from, plus a handful
//! of built-in presets. Anything more opinionated should live in the
//! screen that uses it.
//!
//! # Philosophy
//!
//! - Sane defaults are always the best first experience.
//! - A preset name (e.g. `terminal_blue`) picks the whole palette.
//! - Individual color roles can be overridden per-key in the settings
//!   file (`theme.primary=cyan`, etc.).
//! - Unknown preset names and unparseable colors **fall back to the
//!   default palette** — the app never crashes or shouts because of
//!   theme config.
//!
//! The format is deliberately plain-text key=value to stay consistent
//! with the rest of the settings file and avoid adding a JSON
//! dependency. A future pass may migrate to a richer format.

use ratatui::style::Color;

/// Names for the built-in presets. Parsing is case-insensitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    Default,
    TerminalBlue,
    Matrix,
    RetroDanger,
}

impl ThemePreset {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "default" => Some(Self::Default),
            "terminal_blue" | "terminal-blue" | "blue" => Some(Self::TerminalBlue),
            "matrix" => Some(Self::Matrix),
            "retro_danger" | "retro-danger" | "danger" => Some(Self::RetroDanger),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::TerminalBlue => "terminal_blue",
            Self::Matrix => "matrix",
            Self::RetroDanger => "retro_danger",
        }
    }

    /// Human-friendly display label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::TerminalBlue => "Terminal Blue",
            Self::Matrix => "Matrix",
            Self::RetroDanger => "Retro Danger",
        }
    }

    /// Palette this preset produces.
    pub fn palette(self) -> Theme {
        match self {
            Self::Default => Theme {
                preset: self,
                primary: Color::Cyan,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                muted: Color::DarkGray,
                cactus: Color::DarkGray,
                highlight: Color::White,
            },
            Self::TerminalBlue => Theme {
                preset: self,
                primary: Color::Blue,
                success: Color::Cyan,
                warning: Color::Yellow,
                error: Color::LightRed,
                muted: Color::DarkGray,
                cactus: Color::Blue,
                highlight: Color::LightBlue,
            },
            Self::Matrix => Theme {
                preset: self,
                primary: Color::Green,
                success: Color::LightGreen,
                warning: Color::Yellow,
                error: Color::Red,
                muted: Color::DarkGray,
                cactus: Color::Green,
                highlight: Color::LightGreen,
            },
            Self::RetroDanger => Theme {
                preset: self,
                primary: Color::Red,
                success: Color::Yellow,
                warning: Color::LightYellow,
                error: Color::LightRed,
                muted: Color::DarkGray,
                cactus: Color::Red,
                highlight: Color::White,
            },
        }
    }
}

/// The resolved theme a screen reads.
///
/// Screens should reference *roles*, not raw colors, whenever a color
/// has semantic meaning (an error is always `theme.error`, a success
/// tick is always `theme.success`). Purely decorative color choices
/// can still use raw `Color::…` values.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub preset: ThemePreset,
    pub primary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub muted: Color,
    pub cactus: Color,
    pub highlight: Color,
}

impl Default for Theme {
    fn default() -> Self {
        ThemePreset::Default.palette()
    }
}

impl Theme {
    /// Whether any individual role has been overridden away from the
    /// base preset. Useful for the Settings screen to show a
    /// "custom overrides active" hint.
    pub fn has_overrides(&self) -> bool {
        let base = self.preset.palette();
        base.primary != self.primary
            || base.success != self.success
            || base.warning != self.warning
            || base.error != self.error
            || base.muted != self.muted
            || base.cactus != self.cactus
            || base.highlight != self.highlight
    }

    /// Build a theme from config lines.
    ///
    /// Honors:
    ///   theme=<preset>
    ///   theme.primary=<color>
    ///   theme.success=<color>
    ///   theme.warning=<color>
    ///   theme.error=<color>
    ///   theme.muted=<color>
    ///   theme.cactus=<color>
    ///   theme.highlight=<color>
    ///
    /// Unknown presets and unparseable colors fall back silently.
    pub fn from_config_lines<'a, I: IntoIterator<Item = &'a str>>(lines: I) -> Self {
        let mut preset = ThemePreset::Default;
        let mut overrides: [(&str, Option<Color>); 7] = [
            ("theme.primary", None),
            ("theme.success", None),
            ("theme.warning", None),
            ("theme.error", None),
            ("theme.muted", None),
            ("theme.cactus", None),
            ("theme.highlight", None),
        ];

        for raw in lines {
            let line = raw.trim();
            if let Some(value) = line.strip_prefix("theme=") {
                if let Some(p) = ThemePreset::parse(value) {
                    preset = p;
                }
                continue;
            }
            for (key, slot) in overrides.iter_mut() {
                if let Some(v) = line.strip_prefix(&format!("{key}=")) {
                    if let Some(color) = parse_color(v) {
                        *slot = Some(color);
                    }
                }
            }
        }

        let mut theme = preset.palette();
        theme.preset = preset;
        if let Some(c) = overrides[0].1 { theme.primary = c; }
        if let Some(c) = overrides[1].1 { theme.success = c; }
        if let Some(c) = overrides[2].1 { theme.warning = c; }
        if let Some(c) = overrides[3].1 { theme.error = c; }
        if let Some(c) = overrides[4].1 { theme.muted = c; }
        if let Some(c) = overrides[5].1 { theme.cactus = c; }
        if let Some(c) = overrides[6].1 { theme.highlight = c; }
        theme
    }
}

/// Parse a color name into a [`Color`].
///
/// Accepts common color names and returns `None` for anything else
/// (caller falls back to the preset default).
pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().to_lowercase();
    Some(match s.as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" | "purple" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "dark_gray" | "dark-gray" | "dim" => Color::DarkGray,
        "lightred" | "light_red" => Color::LightRed,
        "lightgreen" | "light_green" => Color::LightGreen,
        "lightyellow" | "light_yellow" => Color::LightYellow,
        "lightblue" | "light_blue" => Color::LightBlue,
        "lightmagenta" | "light_magenta" => Color::LightMagenta,
        "lightcyan" | "light_cyan" => Color::LightCyan,
        "white" => Color::White,
        _ => return None,
    })
}
