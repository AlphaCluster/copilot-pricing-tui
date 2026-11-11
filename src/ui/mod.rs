mod catalog;
mod compare;
mod estimate;
mod help;
pub mod theme;
mod widgets;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::app::{App, Tab};

pub fn draw(frame: &mut Frame, app: &mut App) {
    theme::set_current(app.theme_id);
    let area = frame.area();
    frame.render_widget(Block::default().style(theme::base()), area);

    let [header, tabs, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Fill(1),
        Constraint::Length(2),
    ])
    .areas(area);

    draw_header(frame, app, header);
    draw_tabs(frame, app, tabs);

    match app.tab {
        Tab::Catalog => catalog::draw(frame, app, body),
        Tab::Compare => compare::draw(frame, app, body),
        Tab::Estimate => estimate::draw(frame, app, body),
    }

    draw_footer(frame, app, footer);

    if app.show_help {
        help::draw(frame, area);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    if area.width == 0 {
        return;
    }
    let (left, right) = header_lines(app, area.width as usize);
    let right_w = (right.width() as u16).min(area.width);
    let left_w = area.width.saturating_sub(right_w);
    let [l, r] =
        Layout::horizontal([Constraint::Length(left_w), Constraint::Length(right_w)]).areas(area);
    frame.render_widget(Paragraph::new(left).style(theme::base()), l);
    frame.render_widget(
        Paragraph::new(right)
            .style(theme::base())
            .alignment(Alignment::Right),
        r,
    );
}

#[derive(Clone, Copy)]
enum HeaderLeft {
    /// Brand, source line, refresh hint, and theme.
    Full,
    /// Brand, stamp, refresh hint, and theme.
    Refresh,
    /// Brand, stamp, and theme.
    Stamp,
    /// Brand and theme.
    Theme,
    /// Brand only.
    Brand,
}

#[derive(Clone, Copy)]
enum HeaderRight {
    /// Plan, price, allowance, and the long selection hint.
    Full,
    /// Plan, price, allowance, and a short selection label.
    Price,
    /// Plan, compact price, and a short selection label.
    Compact,
    /// Plan badge only.
    Plan,
}

fn header_lines(app: &App, width: usize) -> (Line<'static>, Line<'static>) {
    const LADDER: [(HeaderLeft, HeaderRight); 10] = [
        (HeaderLeft::Full, HeaderRight::Full),
        (HeaderLeft::Refresh, HeaderRight::Full),
        (HeaderLeft::Stamp, HeaderRight::Full),
        (HeaderLeft::Full, HeaderRight::Price),
        (HeaderLeft::Refresh, HeaderRight::Price),
        (HeaderLeft::Stamp, HeaderRight::Price),
        (HeaderLeft::Stamp, HeaderRight::Compact),
        (HeaderLeft::Theme, HeaderRight::Compact),
        (HeaderLeft::Brand, HeaderRight::Compact),
        (HeaderLeft::Brand, HeaderRight::Plan),
    ];
    for (left_kind, right_kind) in LADDER {
        let left = header_left(app, left_kind);
        let right = header_right(app, right_kind);
        if left.width() + right.width() <= width {
            return (left, right);
        }
    }
    (header_left(app, HeaderLeft::Brand), Line::default())
}

fn header_left(app: &App, kind: HeaderLeft) -> Line<'static> {
    let mut spans = vec![
        Span::styled(
            "  COPILOT",
            Style::new()
                .fg(theme::accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" PRICING", theme::title()),
    ];
    match kind {
        HeaderLeft::Full => {
            spans.push(Span::styled(header_source(app), theme::muted()));
            spans.push(Span::styled("  ·  ", theme::muted()));
            spans.push(widgets::badge(app.theme_id.label(), theme::surface2()));
        }
        HeaderLeft::Refresh => {
            spans.push(Span::styled(
                format!("  {} · u to refresh", header_stamp(app)),
                theme::muted(),
            ));
            spans.push(Span::raw("  "));
            spans.push(widgets::badge(app.theme_id.label(), theme::surface2()));
        }
        HeaderLeft::Stamp => {
            spans.push(Span::styled(
                format!("  {}", header_stamp(app)),
                theme::muted(),
            ));
            spans.push(Span::raw("  "));
            spans.push(widgets::badge(app.theme_id.label(), theme::surface2()));
        }
        HeaderLeft::Theme => {
            spans.push(Span::raw("  "));
            spans.push(widgets::badge(app.theme_id.label(), theme::surface2()));
        }
        HeaderLeft::Brand => {}
    }
    Line::from(spans)
}

fn header_right(app: &App, kind: HeaderRight) -> Line<'static> {
    let plan = app.plan();
    let mut spans = vec![Span::raw("  "), widgets::badge(&plan.name, theme::accent())];
    match kind {
        HeaderRight::Full => {
            spans.push(Span::styled(price_full(plan), theme::muted()));
            spans.push(Span::styled(
                selection_label(app, true),
                Style::new().fg(theme::teal()),
            ));
        }
        HeaderRight::Price => {
            spans.push(Span::styled(price_full(plan), theme::muted()));
            spans.push(Span::styled(
                selection_label(app, false),
                Style::new().fg(theme::teal()),
            ));
        }
        HeaderRight::Compact => {
            spans.push(Span::styled(price_compact(plan), theme::muted()));
            spans.push(Span::styled(
                selection_label(app, false),
                Style::new().fg(theme::teal()),
            ));
        }
        HeaderRight::Plan => {}
    }
    spans.push(Span::raw(" "));
    Line::from(spans)
}

fn price_full(plan: &crate::data::Plan) -> String {
    format!(
        "  ${:.0}/mo  ·  {} credits  (${:.0})   ",
        plan.price_usd,
        widgets::tokens(u64::from(plan.credits)),
        plan.usd_allowance()
    )
}

fn price_compact(plan: &crate::data::Plan) -> String {
    format!(
        "  ${:.0}/mo · {} cr  ",
        plan.price_usd,
        widgets::tokens(u64::from(plan.credits))
    )
}

fn selection_label(app: &App, verbose: bool) -> String {
    let selected = app.selected.len();
    if selected == 0 {
        if verbose {
            "no selection · compare uses visible set".to_string()
        } else {
            "no selection".to_string()
        }
    } else {
        format!("{selected} selected")
    }
}

fn header_stamp(app: &App) -> String {
    match app.catalog.fetched_at.as_deref() {
        Some("bundled") | None => "bundled".to_string(),
        Some(stamp) => stamp.to_string(),
    }
}

fn header_source(app: &App) -> String {
    match app.catalog.fetched_at.as_deref() {
        Some("bundled") | None => "  bundled rates · press u to refresh".into(),
        Some(stamp) => format!("  from GitHub docs · {stamp} · u to refresh"),
    }
}

fn draw_tabs(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|tab| Line::from(format!("  {}  ", tab.label())))
        .collect();
    let selected = Tab::ALL.iter().position(|t| *t == app.tab).unwrap_or(0);
    let tabs = Tabs::new(titles)
        .select(selected)
        .style(Style::new().fg(theme::dim()).bg(theme::bg()))
        .highlight_style(
            Style::new()
                .fg(theme::fg())
                .bg(theme::surface2())
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled("│", Style::new().fg(theme::border())))
        .padding(" ", " ")
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::new().fg(theme::border()))
                .style(theme::base()),
        );
    frame.render_widget(tabs, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let pairs: &[(&str, &str)] = match app.tab {
        Tab::Catalog if app.searching => &[
            ("enter", "apply"),
            ("esc", "clear"),
            ("type", "filter models"),
        ],
        Tab::Catalog => &[
            ("space", "select"),
            ("enter", "compare"),
            ("x", "clear"),
            ("t", "theme"),
            ("u", "update"),
            ("?", "help"),
            ("q", "quit"),
        ],
        Tab::Compare => &[
            ("m", "metric"),
            ("v", "layout"),
            ("o", "columns"),
            ("x", "clear"),
            ("?", "help"),
            ("q", "quit"),
        ],
        Tab::Estimate => &[
            ("n", "preset"),
            ("h l", "adjust"),
            ("p", "plan"),
            ("u", "update"),
            ("?", "help"),
            ("q", "quit"),
        ],
    };
    let footer = if let Some(status) = &app.status {
        let style = if app.status_error {
            Style::new().fg(theme::red())
        } else {
            Style::new().fg(theme::teal())
        };
        Paragraph::new(Line::from(Span::styled(format!("  {status}"), style)))
    } else {
        Paragraph::new(widgets::hint_line(pairs))
    };
    frame.render_widget(
        footer.block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::new().fg(theme::border()))
                .style(theme::base()),
        ),
        area,
    );
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::app::App;

    fn screen(width: u16, height: u16, help: bool) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut term = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.show_help = help;
        term.draw(|frame| super::draw(frame, &mut app)).unwrap();
        let buf = term.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn header_fits_and_help_keys_stay_in_columns() {
        for width in [80, 100, 120, 160] {
            let rows = screen(width, 40, true);
            let header = &rows[0];
            assert!(
                header.contains("COPILOT"),
                "width {width} lost the brand: {header:?}"
            );
            assert!(
                header.contains("Pro+"),
                "width {width} lost the plan: {header:?}"
            );
            assert!(
                header.contains("no selection"),
                "width {width} clipped the selection: {header:?}"
            );
            assert!(
                !header.contains('┌'),
                "width {width} help covers the title bar: {header:?}"
            );
            let orphan = rows.iter().any(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("bottom") || trimmed.starts_with("select / deselect")
            });
            assert!(
                !orphan,
                "width {width} wrapped a help binding mid-pair:\n{}",
                rows.join("\n")
            );
            assert!(
                rows.iter().any(|line| line.contains("top / bottom")),
                "width {width} lost the movement binding:\n{}",
                rows.join("\n")
            );
            assert!(
                rows.iter()
                    .any(|line| line.contains("Press Esc or ? to close")),
                "width {width} clipped the help footer"
            );
            assert!(
                rows.iter().any(|line| line.contains("shown")),
                "width {width} clipped the filter count"
            );
        }
    }
}
