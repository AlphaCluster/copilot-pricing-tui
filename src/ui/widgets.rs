use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::theme;

pub fn money(value: f64) -> String {
    if value <= 0.0 {
        "—".into()
    } else if value >= 1.0 || (value * 100.0).fract().abs() < 1e-9 {
        format!("${value:.2}")
    } else {
        format!("${value:.3}")
    }
}

pub fn money_opt(value: Option<f64>) -> String {
    match value {
        Some(v) => money(v),
        None => "—".into(),
    }
}

pub fn credits(value: f64) -> String {
    if !value.is_finite() {
        "∞".into()
    } else if value >= 100.0 {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else if value >= 0.01 {
        format!("{value:.2}")
    } else if value > 0.0 {
        "<0.01".into()
    } else {
        "0".into()
    }
}

pub fn count(value: f64) -> String {
    if !value.is_finite() {
        "∞".into()
    } else if value >= 100.0 {
        format!("{:.0}", value.floor())
    } else if value >= 10.0 {
        format!("{value:.0}")
    } else if value >= 1.0 {
        format!("{value:.1}")
    } else if value > 0.0 {
        format!("{value:.2}")
    } else {
        "0".into()
    }
}

pub fn tokens(value: u64) -> String {
    let s = value.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

pub fn pct(value: f64) -> String {
    if !value.is_finite() {
        "—".into()
    } else if value >= 10.0 {
        format!("{value:.0}%")
    } else if value >= 1.0 {
        format!("{value:.1}%")
    } else {
        format!("{value:.2}%")
    }
}

pub fn bar_line(ratio: f64, width: usize) -> Line<'static> {
    let ratio = if ratio.is_finite() {
        ratio.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if width == 0 {
        return Line::default();
    }
    let filled = ((ratio * width as f64).round() as usize).min(width);
    let empty = width - filled;
    Line::from(vec![
        Span::styled("█".repeat(filled), Style::new().fg(theme::heat(ratio))),
        Span::styled("░".repeat(empty), Style::new().fg(theme::border())),
    ])
}

pub fn key(label: &str, action: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            format!(" {label} "),
            Style::new()
                .fg(theme::bg())
                .bg(theme::surface2())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {action}  "), theme::muted()),
    ]
}

pub fn hint_line(pairs: &[(&str, &str)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, (label, action)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("·", theme::muted()));
        }
        spans.extend(key(label, action));
    }
    Line::from(spans)
}

pub fn badge(text: &str, color: ratatui::style::Color) -> Span<'static> {
    Span::styled(
        format!(" {text} "),
        Style::new()
            .fg(theme::bg())
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

pub fn promo_badge() -> Span<'static> {
    badge("PROMO", theme::yellow())
}
