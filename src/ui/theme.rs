//! Runtime themes. Colors come from ratatui's Tailwind palette where it fits,
//! plus a few well-known terminal palettes.

use std::cell::Cell;

use ratatui::style::palette::tailwind::{
    AMBER, BLUE, CYAN, EMERALD, ORANGE as TW_ORANGE, PINK as TW_PINK, PURPLE as TW_PURPLE,
    RED as TW_RED, SLATE,
};
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

use crate::data::{Category, Provider};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeId {
    #[default]
    GitHub,
    Dracula,
    Catppuccin,
    Nord,
    TokyoNight,
    Gruvbox,
    Solarized,
    OneDark,
    Monokai,
    Light,
}

impl ThemeId {
    pub const ALL: [ThemeId; 10] = [
        ThemeId::GitHub,
        ThemeId::Dracula,
        ThemeId::Catppuccin,
        ThemeId::Nord,
        ThemeId::TokyoNight,
        ThemeId::Gruvbox,
        ThemeId::Solarized,
        ThemeId::OneDark,
        ThemeId::Monokai,
        ThemeId::Light,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeId::GitHub => "GitHub",
            ThemeId::Dracula => "Dracula",
            ThemeId::Catppuccin => "Catppuccin",
            ThemeId::Nord => "Nord",
            ThemeId::TokyoNight => "Tokyo Night",
            ThemeId::Gruvbox => "Gruvbox",
            ThemeId::Solarized => "Solarized",
            ThemeId::OneDark => "One Dark",
            ThemeId::Monokai => "Monokai",
            ThemeId::Light => "Light",
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|id| *id == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let i = Self::ALL.iter().position(|id| *id == self).unwrap_or(0);
        Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub fn palette(self) -> Theme {
        match self {
            ThemeId::GitHub => Theme {
                bg: Color::Rgb(13, 17, 23),
                surface: Color::Rgb(22, 27, 34),
                surface2: Color::Rgb(33, 38, 45),
                border: Color::Rgb(48, 54, 61),
                fg: Color::Rgb(230, 237, 243),
                dim: Color::Rgb(139, 148, 158),
                accent: Color::Rgb(88, 166, 255),
                green: Color::Rgb(63, 185, 80),
                yellow: Color::Rgb(210, 153, 34),
                orange: Color::Rgb(219, 109, 40),
                red: Color::Rgb(248, 81, 73),
                purple: Color::Rgb(188, 140, 255),
                pink: Color::Rgb(255, 123, 184),
                teal: Color::Rgb(57, 197, 211),
            },
            ThemeId::Dracula => Theme {
                bg: Color::Rgb(40, 42, 54),
                surface: Color::Rgb(52, 55, 70),
                surface2: Color::Rgb(68, 71, 90),
                border: Color::Rgb(98, 114, 164),
                fg: Color::Rgb(248, 248, 242),
                dim: Color::Rgb(98, 114, 164),
                accent: Color::Rgb(139, 233, 253),
                green: Color::Rgb(80, 250, 123),
                yellow: Color::Rgb(241, 250, 140),
                orange: Color::Rgb(255, 184, 108),
                red: Color::Rgb(255, 85, 85),
                purple: Color::Rgb(189, 147, 249),
                pink: Color::Rgb(255, 121, 198),
                teal: Color::Rgb(139, 233, 253),
            },
            ThemeId::Catppuccin => Theme {
                bg: Color::Rgb(30, 30, 46),
                surface: Color::Rgb(24, 24, 37),
                surface2: Color::Rgb(49, 50, 68),
                border: Color::Rgb(69, 71, 90),
                fg: Color::Rgb(205, 214, 244),
                dim: Color::Rgb(108, 112, 134),
                accent: Color::Rgb(137, 180, 250),
                green: Color::Rgb(166, 227, 161),
                yellow: Color::Rgb(249, 226, 175),
                orange: Color::Rgb(250, 179, 135),
                red: Color::Rgb(243, 139, 168),
                purple: Color::Rgb(203, 166, 247),
                pink: Color::Rgb(245, 194, 231),
                teal: Color::Rgb(148, 226, 213),
            },
            ThemeId::Nord => Theme {
                bg: Color::Rgb(46, 52, 64),
                surface: Color::Rgb(59, 66, 82),
                surface2: Color::Rgb(67, 76, 94),
                border: Color::Rgb(76, 86, 106),
                fg: Color::Rgb(236, 239, 244),
                dim: Color::Rgb(123, 136, 161),
                accent: Color::Rgb(136, 192, 208),
                green: Color::Rgb(163, 190, 140),
                yellow: Color::Rgb(235, 203, 139),
                orange: Color::Rgb(208, 135, 112),
                red: Color::Rgb(191, 97, 106),
                purple: Color::Rgb(180, 142, 173),
                pink: Color::Rgb(180, 142, 173),
                teal: Color::Rgb(143, 188, 187),
            },
            ThemeId::TokyoNight => Theme {
                bg: Color::Rgb(26, 27, 38),
                surface: Color::Rgb(22, 22, 30),
                surface2: Color::Rgb(41, 46, 66),
                border: Color::Rgb(59, 66, 97),
                fg: Color::Rgb(192, 202, 245),
                dim: Color::Rgb(86, 95, 137),
                accent: Color::Rgb(122, 162, 247),
                green: Color::Rgb(158, 206, 106),
                yellow: Color::Rgb(224, 175, 104),
                orange: Color::Rgb(255, 158, 100),
                red: Color::Rgb(247, 118, 142),
                purple: Color::Rgb(187, 154, 247),
                pink: Color::Rgb(187, 154, 247),
                teal: Color::Rgb(125, 207, 255),
            },
            ThemeId::Gruvbox => Theme {
                bg: Color::Rgb(40, 40, 40),
                surface: Color::Rgb(60, 56, 54),
                surface2: Color::Rgb(80, 73, 69),
                border: Color::Rgb(102, 92, 84),
                fg: Color::Rgb(235, 219, 178),
                dim: Color::Rgb(146, 131, 116),
                accent: Color::Rgb(131, 165, 152),
                green: Color::Rgb(184, 187, 38),
                yellow: Color::Rgb(250, 189, 47),
                orange: Color::Rgb(254, 128, 25),
                red: Color::Rgb(251, 73, 52),
                purple: Color::Rgb(211, 134, 155),
                pink: Color::Rgb(211, 134, 155),
                teal: Color::Rgb(142, 192, 124),
            },
            ThemeId::Solarized => Theme {
                bg: Color::Rgb(0, 43, 54),
                surface: Color::Rgb(7, 54, 66),
                surface2: Color::Rgb(9, 67, 82),
                border: Color::Rgb(88, 110, 117),
                fg: Color::Rgb(238, 232, 213),
                dim: Color::Rgb(131, 148, 150),
                accent: Color::Rgb(38, 139, 210),
                green: Color::Rgb(133, 153, 0),
                yellow: Color::Rgb(181, 137, 0),
                orange: Color::Rgb(203, 75, 22),
                red: Color::Rgb(220, 50, 47),
                purple: Color::Rgb(108, 113, 196),
                pink: Color::Rgb(211, 54, 130),
                teal: Color::Rgb(42, 161, 152),
            },
            ThemeId::OneDark => Theme {
                bg: Color::Rgb(40, 44, 52),
                surface: Color::Rgb(33, 37, 43),
                surface2: Color::Rgb(62, 68, 81),
                border: Color::Rgb(75, 82, 99),
                fg: Color::Rgb(171, 178, 191),
                dim: Color::Rgb(92, 99, 112),
                accent: Color::Rgb(97, 175, 239),
                green: Color::Rgb(152, 195, 121),
                yellow: Color::Rgb(229, 192, 123),
                orange: Color::Rgb(209, 154, 102),
                red: Color::Rgb(224, 108, 117),
                purple: Color::Rgb(198, 120, 221),
                pink: Color::Rgb(198, 120, 221),
                teal: Color::Rgb(86, 182, 194),
            },
            ThemeId::Monokai => Theme {
                bg: Color::Rgb(39, 40, 34),
                surface: Color::Rgb(30, 31, 28),
                surface2: Color::Rgb(62, 61, 50),
                border: Color::Rgb(117, 113, 94),
                fg: Color::Rgb(248, 248, 242),
                dim: Color::Rgb(117, 113, 94),
                accent: Color::Rgb(102, 217, 239),
                green: Color::Rgb(166, 226, 46),
                yellow: Color::Rgb(230, 219, 116),
                orange: Color::Rgb(253, 151, 31),
                red: Color::Rgb(249, 38, 114),
                purple: Color::Rgb(174, 129, 255),
                pink: Color::Rgb(249, 38, 114),
                teal: Color::Rgb(102, 217, 239),
            },
            ThemeId::Light => Theme {
                bg: Color::Rgb(255, 255, 255),
                surface: SLATE.c50,
                surface2: SLATE.c200,
                border: SLATE.c300,
                fg: SLATE.c900,
                dim: SLATE.c500,
                accent: BLUE.c600,
                green: EMERALD.c600,
                yellow: AMBER.c600,
                orange: TW_ORANGE.c600,
                red: TW_RED.c600,
                purple: TW_PURPLE.c600,
                pink: TW_PINK.c600,
                teal: CYAN.c600,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub surface2: Color,
    pub border: Color,
    pub fg: Color,
    pub dim: Color,
    pub accent: Color,
    pub green: Color,
    pub yellow: Color,
    pub orange: Color,
    pub red: Color,
    pub purple: Color,
    pub pink: Color,
    pub teal: Color,
}

thread_local! {
    static CURRENT: Cell<ThemeId> = const { Cell::new(ThemeId::GitHub) };
}

pub fn set_current(id: ThemeId) {
    CURRENT.set(id);
}

pub fn current_id() -> ThemeId {
    CURRENT.get()
}

fn palette() -> Theme {
    CURRENT.get().palette()
}

pub fn bg() -> Color {
    palette().bg
}
pub fn surface() -> Color {
    palette().surface
}
pub fn surface2() -> Color {
    palette().surface2
}
pub fn border() -> Color {
    palette().border
}
pub fn fg() -> Color {
    palette().fg
}
pub fn dim() -> Color {
    palette().dim
}
pub fn accent() -> Color {
    palette().accent
}
pub fn green() -> Color {
    palette().green
}
pub fn yellow() -> Color {
    palette().yellow
}
pub fn orange() -> Color {
    palette().orange
}
pub fn red() -> Color {
    palette().red
}
pub fn purple() -> Color {
    palette().purple
}
pub fn pink() -> Color {
    palette().pink
}
pub fn teal() -> Color {
    palette().teal
}

pub fn base() -> Style {
    Style::new().bg(bg()).fg(fg())
}

pub fn muted() -> Style {
    Style::new().fg(dim())
}

pub fn title() -> Style {
    Style::new().fg(fg()).add_modifier(Modifier::BOLD)
}

pub fn selected_row() -> Style {
    Style::new().bg(surface2()).fg(fg())
}

pub fn provider(provider: Provider) -> Color {
    match provider {
        Provider::OpenAI => green(),
        Provider::Anthropic => orange(),
        Provider::Google => accent(),
        Provider::GitHub => teal(),
        Provider::Microsoft => teal(),
        Provider::Xai => fg(),
        Provider::Moonshot => purple(),
        Provider::Other => dim(),
    }
}

pub fn category(category: Category) -> Color {
    match category {
        Category::Lightweight => green(),
        Category::Versatile => accent(),
        Category::Powerful => pink(),
    }
}

/// Map 0..=1 to a cheap→expensive color.
pub fn heat(ratio: f64) -> Color {
    let ratio = if ratio.is_finite() {
        ratio.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if ratio < 0.25 {
        green()
    } else if ratio < 0.5 {
        teal()
    } else if ratio < 0.75 {
        yellow()
    } else if ratio < 0.9 {
        orange()
    } else {
        red()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_visits_every_theme() {
        let mut id = ThemeId::GitHub;
        let mut seen = Vec::new();
        for _ in 0..ThemeId::ALL.len() {
            seen.push(id);
            id = id.next();
        }
        assert_eq!(id, ThemeId::GitHub);
        assert_eq!(seen.len(), ThemeId::ALL.len());
    }

    #[test]
    fn tailwind_light_theme_uses_framework_palette() {
        let light = ThemeId::Light.palette();
        assert_eq!(light.accent, BLUE.c600);
        assert_eq!(light.surface, SLATE.c50);
    }
}
