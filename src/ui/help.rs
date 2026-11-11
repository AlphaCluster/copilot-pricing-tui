use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

use super::theme;

pub fn draw(frame: &mut Frame, area: Rect) {
    if area.width < 24 || area.height < 8 {
        return;
    }

    // Keep the title bar and the footer hint line visible.
    let body = if area.height >= 16 {
        Rect {
            x: area.x,
            y: area.y + 2,
            width: area.width,
            height: area.height.saturating_sub(4),
        }
    } else {
        area
    };

    let outer_w = body.width.min(76);
    let pad_x: u16 = 2;
    let pad_y: u16 = if body.height >= 28 { 1 } else { 0 };
    let inner_w = outer_w.saturating_sub(2 + pad_x * 2) as usize;
    let lines = help_lines(inner_w);
    let chrome_y = 2 + pad_y * 2;
    let outer_h = (lines.len() as u16)
        .saturating_add(chrome_y)
        .min(body.height)
        .max(3);
    let popup = Rect {
        x: body.x + (body.width.saturating_sub(outer_w)) / 2,
        y: body.y + (body.height.saturating_sub(outer_h)) / 2,
        width: outer_w,
        height: outer_h,
    };

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(Span::styled(" Help ", theme::title()))
                .borders(Borders::ALL)
                .border_style(Style::new().fg(theme::accent()))
                .style(Style::new().bg(theme::surface()).fg(theme::fg()))
                .padding(Padding::new(pad_x, pad_x, pad_y, pad_y)),
        ),
        popup,
    );
}

fn help_lines(width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        "GitHub Copilot model pricing",
        Style::new().fg(theme::fg()).add_modifier(Modifier::BOLD),
    )));
    lines.extend(wrap_muted(
        "Prices are USD per 1M tokens. 1 AI credit = $0.01. Completions stay unlimited.",
        width,
    ));
    lines.push(Line::raw(""));

    lines.push(section("Movement"));
    lines.extend(key_rows(
        &[
            ("1 2 3 / Tab", "switch views"),
            ("j k / ↑ ↓", "move"),
            ("g G", "top / bottom"),
            ("q Esc", "quit"),
        ],
        width,
    ));
    lines.push(Line::raw(""));

    lines.push(section("Catalog"));
    lines.extend(key_rows(
        &[
            ("Space", "select / deselect"),
            ("Enter", "compare selection"),
            ("a", "select visible"),
            ("x", "clear comparison"),
            ("/", "search"),
            ("f F", "provider"),
            ("c C", "category"),
            ("s S", "sort / reverse"),
        ],
        width,
    ));
    lines.push(Line::raw(""));

    lines.push(section("Compare & estimate"));
    lines.extend(key_rows(
        &[
            ("m", "compare metric"),
            ("v", "column layout"),
            ("o", "hide/show columns"),
            ("b", "extra benches"),
            ("p P", "cycle plan"),
            ("d", "10% auto discount"),
            ("t T", "cycle theme"),
            ("u", "refresh docs"),
            ("?", "this help"),
        ],
        width,
    ));
    lines.push(Line::raw(""));

    lines.extend(wrap_muted(
        "Plans with an allowance: Pro, Pro+, Max, Business, Enterprise. Copilot access includes none.",
        width,
    ));
    lines.extend(wrap_muted(
        "d is the 10% auto-selection discount. Copilot Fusion's 30% (through 2026-11-30) is separate.",
        width,
    ));
    lines.extend(wrap_muted(
        "Coding chat is 12k input / 6k cached / 1.5k cache-write / 2k output.",
        width,
    ));
    lines.extend(wrap_muted(
        "Rates from GitHub docs. SWE-bench, Terminal-Bench, and LiveCodeBench from vals.ai.",
        width,
    ));
    lines.push(Line::from(Span::styled(
        "Press Esc or ? to close",
        Style::new().fg(theme::accent()),
    )));
    lines
}

/// Two key/action pairs per row when they fit, otherwise one. Columns share a width so keys line up.
fn key_rows(pairs: &[(&str, &str)], width: usize) -> Vec<Line<'static>> {
    // Shared by every section so the two columns stay on one grid.
    let key_w = 12;
    let action_w = 18;
    let cols = if width >= key_w * 2 + action_w * 2 + 4 {
        2
    } else {
        1
    };
    let action_w = if cols == 2 {
        action_w
    } else {
        width.saturating_sub(key_w + 1).max(1)
    };
    pairs
        .chunks(cols)
        .map(|chunk| {
            let mut spans = Vec::new();
            for (i, (key, action)) in chunk.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::raw("  "));
                }
                spans.push(Span::styled(
                    pad(key, key_w),
                    Style::new().fg(theme::fg()).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" "));
                spans.push(Span::styled(pad(action, action_w), theme::muted()));
            }
            Line::from(spans)
        })
        .collect()
}

fn section(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::new()
            .fg(theme::accent())
            .add_modifier(Modifier::BOLD),
    ))
}

fn wrap_muted(text: &str, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = break_word(word, width, &mut lines);
            continue;
        }
        let next_w = Line::from(current.as_str()).width() + 1 + Line::from(word).width();
        if next_w <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(muted_line(std::mem::take(&mut current)));
            current = break_word(word, width, &mut lines);
        }
    }
    if !current.is_empty() {
        lines.push(muted_line(current));
    }
    lines
}

/// Push every full-width slice of `word` except the tail, which is returned.
fn break_word(word: &str, width: usize, lines: &mut Vec<Line<'static>>) -> String {
    if width == 0 {
        return String::new();
    }
    let mut rest = word.to_string();
    while Line::from(rest.as_str()).width() > width {
        let (head, tail) = split_width(&rest, width);
        lines.push(muted_line(head));
        rest = tail;
    }
    rest
}

fn muted_line(text: String) -> Line<'static> {
    Line::from(Span::styled(text, theme::muted()))
}

fn pad(text: &str, width: usize) -> String {
    let w = Line::from(text).width();
    if w == width {
        text.to_string()
    } else if w < width {
        format!("{text}{}", " ".repeat(width - w))
    } else {
        split_width(text, width).0
    }
}

fn split_width(text: &str, width: usize) -> (String, String) {
    let mut head = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let cw = Line::from(ch.to_string()).width().max(1);
        if used + cw > width {
            let tail = text[head.len()..].to_string();
            return (head, tail);
        }
        head.push(ch);
        used += cw;
    }
    (head, String::new())
}
