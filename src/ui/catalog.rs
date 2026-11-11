use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table};

use super::{theme, widgets};
use crate::app::App;
use crate::calc::interaction_cost;
use crate::data::TYPICAL_USAGE;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let [filters, table, detail] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(9),
    ])
    .areas(area);

    draw_filters(frame, app, filters);
    draw_table(frame, app, table);
    draw_detail(frame, app, detail);
}

fn draw_filters(frame: &mut Frame, app: &App, area: Rect) {
    let provider = app
        .provider_filter
        .map(|p| p.label())
        .unwrap_or("all providers");
    let category = app
        .category_filter
        .map(|c| c.label())
        .unwrap_or("all categories");
    let dir = if app.sort_desc { "↓" } else { "↑" };
    let search = if app.searching {
        format!("/{}", app.search)
    } else if app.search.is_empty() {
        "/ to search".into()
    } else {
        format!("filter: {}", app.search)
    };

    let count = format!(
        "{} of {} shown",
        app.visible_models().len(),
        app.catalog.models.len()
    );
    let line = filter_line(
        area.width as usize,
        provider,
        category,
        &format!("sort {dir} {}", app.sort.label()),
        &search,
        app.searching,
        &count,
    );

    frame.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::new().fg(theme::border()))
                .style(theme::base()),
        ),
        area,
    );
}

fn filter_line(
    width: usize,
    provider: &str,
    category: &str,
    sort: &str,
    search: &str,
    searching: bool,
    count: &str,
) -> Line<'static> {
    let search_style = if searching {
        Style::new()
            .fg(theme::accent())
            .add_modifier(Modifier::BOLD)
    } else {
        theme::muted()
    };
    let assemble = |show_category: bool, show_search: bool, show_count: bool| {
        let mut spans = Vec::new();
        spans.push(Span::styled("  f ", theme::muted()));
        spans.push(widgets::badge(provider, theme::surface2()));
        if show_category {
            spans.push(Span::styled("  c ", theme::muted()));
            spans.push(widgets::badge(category, theme::surface2()));
        }
        spans.push(Span::styled("  s ", theme::muted()));
        spans.push(widgets::badge(sort, theme::surface2()));
        if show_search {
            spans.push(Span::raw("   "));
            spans.push(Span::styled(search.to_string(), search_style));
        }
        if show_count {
            spans.push(Span::raw("    "));
            spans.push(Span::styled(count.to_string(), theme::muted()));
        }
        Line::from(spans)
    };
    // Idle search hint goes first. Then category, then the count.
    let candidates = [
        assemble(true, true, true),
        assemble(true, searching, true),
        assemble(false, searching, true),
        assemble(false, searching, false),
        assemble(false, false, false),
    ];
    candidates
        .into_iter()
        .find(|line| line.width() <= width)
        .unwrap_or_else(|| assemble(false, false, false))
}

fn draw_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let visible: Vec<crate::data::Model> = app.visible_models().into_iter().cloned().collect();
    let max_typical = visible
        .iter()
        .map(|m| interaction_cost(m, TYPICAL_USAGE, false).usd)
        .fold(0.0_f64, f64::max)
        .max(1e-9);

    let wide = area.width >= 118;
    let medium = area.width >= 96;

    let header = if wide {
        Row::new([
            Cell::from(""),
            Cell::from("Model"),
            Cell::from("Provider"),
            Cell::from("Class"),
            Cell::from("SWE-V"),
            Cell::from("Input"),
            Cell::from("Cached"),
            Cell::from("Write"),
            Cell::from("Output"),
            Cell::from("Coding chat"),
        ])
        .style(Style::new().fg(theme::dim()).add_modifier(Modifier::BOLD))
        .height(1)
    } else if medium {
        Row::new([
            Cell::from(""),
            Cell::from("Model"),
            Cell::from("Provider"),
            Cell::from("SWE-V"),
            Cell::from("Input"),
            Cell::from("Output"),
            Cell::from("Coding chat"),
        ])
        .style(Style::new().fg(theme::dim()).add_modifier(Modifier::BOLD))
    } else {
        Row::new([
            Cell::from(""),
            Cell::from("Model"),
            Cell::from("SWE-V"),
            Cell::from("Output"),
            Cell::from("Coding chat"),
        ])
        .style(Style::new().fg(theme::dim()).add_modifier(Modifier::BOLD))
    };

    let rows = visible.iter().map(|model| {
        let on = app.is_selected(&model.id);
        let mark = if on { "●" } else { "○" };
        let mark_color = if on { theme::accent() } else { theme::dim() };
        let typical = interaction_cost(model, TYPICAL_USAGE, false);
        let bar_w = if wide { 10 } else { 8 };
        let bar = widgets::bar_line(typical.usd / max_typical, bar_w);
        let typical_with_bar = Cell::from(Line::from({
            let mut spans = vec![Span::styled(
                format!("{:>6} cr ", widgets::credits(typical.credits)),
                Style::new().fg(theme::heat(typical.usd / max_typical)),
            )];
            spans.extend(bar.spans);
            spans
        }));

        let name = Cell::from(Line::from({
            let mut spans = vec![Span::styled(model.label(), Style::new().fg(theme::fg()))];
            if model.is_promo() {
                spans.push(Span::raw(" "));
                spans.push(widgets::promo_badge());
            } else if model.notes.is_some() {
                spans.push(Span::styled("  *", Style::new().fg(theme::yellow())));
            }
            spans
        }));
        let swe = app
            .catalog
            .bench_for(model)
            .and_then(|score| score.swe_verified);
        let swe_cell = Cell::from(Span::styled(
            swe.map(|v| format!("{v:.1}%"))
                .unwrap_or_else(|| "—".into()),
            if swe.is_some() {
                Style::new().fg(theme::teal())
            } else {
                theme::muted()
            },
        ));

        if wide {
            Row::new([
                Cell::from(Span::styled(mark, Style::new().fg(mark_color))),
                name,
                Cell::from(Span::styled(
                    model.provider.label(),
                    Style::new().fg(theme::provider(model.provider)),
                )),
                Cell::from(Span::styled(
                    model.category.short(),
                    Style::new().fg(theme::category(model.category)),
                )),
                swe_cell.clone(),
                Cell::from(Span::styled(widgets::money(model.input), theme::muted())),
                Cell::from(Span::styled(widgets::money(model.cached), theme::muted())),
                Cell::from(Span::styled(
                    widgets::money_opt(model.cache_write),
                    theme::muted(),
                )),
                Cell::from(Span::styled(
                    widgets::money(model.output),
                    Style::new().fg(theme::fg()),
                )),
                typical_with_bar,
            ])
        } else if medium {
            Row::new([
                Cell::from(Span::styled(mark, Style::new().fg(mark_color))),
                name,
                Cell::from(Span::styled(
                    model.provider.label(),
                    Style::new().fg(theme::provider(model.provider)),
                )),
                swe_cell.clone(),
                Cell::from(widgets::money(model.input)),
                Cell::from(widgets::money(model.output)),
                typical_with_bar,
            ])
        } else {
            Row::new([
                Cell::from(Span::styled(mark, Style::new().fg(mark_color))),
                name,
                swe_cell,
                Cell::from(widgets::money(model.output)),
                typical_with_bar,
            ])
        }
        .style(theme::base())
        .height(1)
    });

    let widths: Vec<Constraint> = if wide {
        vec![
            Constraint::Length(2),
            Constraint::Fill(3),
            Constraint::Length(11),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(20),
        ]
    } else if medium {
        vec![
            Constraint::Length(2),
            Constraint::Fill(2),
            Constraint::Length(11),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(20),
        ]
    } else {
        vec![
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(20),
        ]
    };

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(theme::selected_row())
        .highlight_symbol("▸ ")
        .style(theme::base())
        .column_spacing(1)
        .block(
            Block::default()
                .style(theme::base())
                .padding(Padding::horizontal(1)),
        );

    frame.render_stateful_widget(table, area, &mut app.catalog_state);
}

fn draw_detail(frame: &mut Frame, app: &App, area: Rect) {
    let visible: Vec<crate::data::Model> = app.visible_models().into_iter().cloned().collect();
    let Some(model) = visible.get(app.catalog_cursor()) else {
        frame.render_widget(
            Paragraph::new("No models match the current filters.")
                .style(theme::muted())
                .block(detail_block()),
            area,
        );
        return;
    };

    let typical = interaction_cost(model, TYPICAL_USAGE, app.auto_discount);
    let plan = app.plan();
    let fits = crate::calc::interactions_per_plan(typical, plan);
    let max_out = app
        .catalog
        .models
        .iter()
        .map(|m| m.output)
        .fold(0.0, f64::max);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(model.name.as_str(), theme::title()),
            if model.variant.is_empty() {
                Span::raw("")
            } else {
                Span::styled(format!("  {}", model.variant), theme::muted())
            },
            Span::raw("   "),
            Span::styled(
                model.provider.label(),
                Style::new().fg(theme::provider(model.provider)),
            ),
            Span::raw(" · "),
            Span::styled(
                model.category.label(),
                Style::new().fg(theme::category(model.category)),
            ),
            Span::raw(" · "),
            Span::styled(model.status.label(), theme::muted()),
            if model.is_promo() {
                Span::raw("  ")
            } else {
                Span::raw("")
            },
            if model.is_promo() {
                widgets::promo_badge()
            } else {
                Span::raw("")
            },
            if app.is_selected(&model.id) {
                Span::styled("   selected", Style::new().fg(theme::accent()))
            } else {
                Span::raw("")
            },
        ]),
        Line::from(vec![
            Span::styled("  input ", theme::muted()),
            Span::styled(
                format!("{:>7}", widgets::money(model.input)),
                theme::title(),
            ),
            Span::styled("    cached ", theme::muted()),
            Span::styled(
                format!("{:>7}", widgets::money(model.cached)),
                theme::title(),
            ),
            Span::styled("    write ", theme::muted()),
            Span::styled(
                format!("{:>7}", widgets::money_opt(model.cache_write)),
                theme::title(),
            ),
            Span::styled("    output ", theme::muted()),
            Span::styled(
                format!("{:>7}", widgets::money(model.output)),
                theme::title(),
            ),
            Span::styled("   / 1M tokens", theme::muted()),
        ]),
        Line::from({
            let score = app.catalog.bench_for(model);
            let mut spans = vec![Span::styled("  SWE-V ", theme::muted())];
            spans.push(Span::styled(
                score
                    .and_then(|s| s.swe_verified)
                    .map(|v| format!("{v:.1}%"))
                    .unwrap_or_else(|| "—".into()),
                theme::title(),
            ));
            spans.push(Span::styled("    TB 2.1 ", theme::muted()));
            spans.push(Span::styled(
                score
                    .and_then(|s| s.terminal_bench)
                    .map(|v| format!("{v:.1}%"))
                    .unwrap_or_else(|| "—".into()),
                theme::title(),
            ));
            spans.push(Span::styled("    LiveCode ", theme::muted()));
            spans.push(Span::styled(
                score
                    .and_then(|s| s.livecodebench)
                    .map(|v| format!("{v:.1}%"))
                    .unwrap_or_else(|| "—".into()),
                theme::title(),
            ));
            if score.is_some() {
                spans.push(Span::styled("    vals.ai · mini-SWE-agent", theme::muted()));
            }
            spans
        }),
        Line::from({
            let mut spans = vec![Span::styled("  output  ", theme::muted())];
            spans.extend(widgets::bar_line(model.output / max_out, 28).spans);
            spans.push(Span::styled(
                format!(
                    "    coding chat  {}  ({} cr)   ~{} / mo on {}",
                    widgets::money(typical.usd),
                    widgets::credits(typical.credits),
                    widgets::count(fits),
                    plan.name
                ),
                theme::muted(),
            ));
            spans
        }),
    ];

    if let Some(promo) = model.promo_label() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            widgets::promo_badge(),
            Span::styled(format!("  {promo}"), Style::new().fg(theme::yellow())),
        ]));
    } else if let Some(note) = &model.notes {
        lines.push(Line::from(Span::styled(
            format!("  * {note}"),
            Style::new().fg(theme::yellow()),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Space selects this model for Compare and Estimate. Enter opens Compare.",
            theme::muted(),
        )));
    }

    frame.render_widget(Paragraph::new(lines).block(detail_block()), area);
}

fn detail_block() -> Block<'static> {
    Block::default()
        .borders(Borders::TOP)
        .border_style(Style::new().fg(theme::border()))
        .style(theme::base())
        .padding(Padding::horizontal(1))
}
