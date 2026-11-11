use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table};

use super::{theme, widgets};
use crate::app::{App, EstField};
use crate::calc::{interaction_cost, interactions_per_plan, plan_percent};
use crate::data::{Model, SCENARIOS};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let wide = area.width >= 100;
    if wide {
        let [left, right] =
            Layout::horizontal([Constraint::Length(42), Constraint::Fill(1)]).areas(area);
        draw_controls(frame, app, left);
        draw_results(frame, app, right);
    } else {
        let [top, bottom] =
            Layout::vertical([Constraint::Length(14), Constraint::Fill(1)]).areas(area);
        draw_controls(frame, app, top);
        draw_results(frame, app, bottom);
    }
}

fn draw_controls(frame: &mut Frame, app: &App, area: Rect) {
    let scenario = app.scenario();
    let usage = app.usage;
    let max_ref = 400_000_f64.max(usage.total_tokens() as f64);

    let field = |which: EstField, value: u64| {
        let focused = app.est_field == which;
        let marker = if focused { "▸" } else { " " };
        let value_style = if focused {
            Style::new()
                .fg(theme::accent())
                .add_modifier(Modifier::BOLD)
        } else {
            theme::title()
        };
        let mut spans = vec![
            Span::styled(format!(" {marker} {:<13}", which.label()), theme::muted()),
            Span::styled(format!("{:>9}  ", widgets::tokens(value)), value_style),
        ];
        spans.extend(widgets::bar_line(value as f64 / max_ref, 12).spans);
        Line::from(spans)
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Scenario  ", theme::muted()),
            widgets::badge(scenario.name, theme::accent()),
            Span::styled("   n next  N prev", theme::muted()),
        ]),
        Line::from(Span::styled(
            format!("  {}", scenario.blurb),
            theme::muted(),
        )),
        Line::raw(""),
        field(EstField::Input, usage.input),
        field(EstField::Cached, usage.cached),
        field(EstField::CacheWrite, usage.cache_write),
        field(EstField::Output, usage.output),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Plan  ", theme::muted()),
            Span::styled(app.plan().name.as_str(), theme::title()),
            Span::styled(
                format!(
                    "  {} credits  (${:.0})  ·  {}",
                    widgets::tokens(u64::from(app.plan().credits)),
                    app.plan().usd_allowance(),
                    app.plan().detail
                ),
                theme::muted(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Auto 10%  ", theme::muted()),
            if app.auto_discount {
                Span::styled(
                    "on",
                    Style::new().fg(theme::green()).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("off", theme::muted())
            },
            Span::styled("   d to toggle", theme::muted()),
        ]),
        Line::from(Span::styled(
            "  , . field    h l adjust    H L coarse",
            theme::muted(),
        )),
    ];

    // Show which preset values still match, if any.
    let matching = SCENARIOS.iter().find(|s| {
        s.input == usage.input
            && s.cached == usage.cached
            && s.cache_write == usage.cache_write
            && s.output == usage.output
    });
    if matching.is_none() {
        lines.push(Line::from(Span::styled(
            "  (custom token mix)",
            Style::new().fg(theme::yellow()),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(Span::styled(" Token mix ", theme::muted()))
                .borders(Borders::ALL)
                .border_style(Style::new().fg(theme::border()))
                .style(theme::base())
                .padding(Padding::new(0, 1, 0, 0)),
        ),
        area,
    );
}

fn draw_results(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut models: Vec<crate::data::Model> = app.compare_set().into_iter().cloned().collect();
    models.sort_by(|a, b| {
        interaction_cost(a, app.usage, app.auto_discount)
            .usd
            .partial_cmp(&interaction_cost(b, app.usage, app.auto_discount).usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if models.is_empty() {
        frame.render_widget(
            Paragraph::new("No models to estimate.")
                .style(theme::muted())
                .block(results_block(app)),
            area,
        );
        return;
    }

    let max = models
        .iter()
        .map(|m| interaction_cost(m, app.usage, app.auto_discount).usd)
        .fold(0.0, f64::max)
        .max(1e-9);
    let plan = app.plan();
    let bar_w = (area.width.saturating_sub(56) as usize).clamp(6, 22);

    let header = Row::new([
        Cell::from("#"),
        Cell::from("Model"),
        Cell::from("USD"),
        Cell::from("Credits"),
        Cell::from("% plan"),
        Cell::from(format!("Fits / {}", plan.name)),
        Cell::from(""),
    ])
    .style(Style::new().fg(theme::dim()).add_modifier(Modifier::BOLD));

    let rows = models.iter().enumerate().map(|(i, model)| {
        let cost = interaction_cost(model, app.usage, app.auto_discount);
        let fits = interactions_per_plan(cost, plan);
        let share = plan_percent(cost, plan);
        let mut bar = vec![Span::raw(" ")];
        bar.extend(widgets::bar_line(cost.usd / max, bar_w).spans);
        Row::new([
            Cell::from(Span::styled(format!("{}", i + 1), theme::muted())),
            Cell::from(model_name(model)),
            Cell::from(Span::styled(
                widgets::money(cost.usd),
                Style::new().fg(theme::heat(cost.usd / max)),
            )),
            Cell::from(widgets::credits(cost.credits)),
            Cell::from(widgets::pct(share)),
            Cell::from(Span::styled(
                widgets::count(fits),
                Style::new().fg(theme::fg()).add_modifier(Modifier::BOLD),
            )),
            Cell::from(Line::from(bar)),
        ])
        .style(theme::base())
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Fill(2),
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Fill(2),
        ],
    )
    .header(header)
    .row_highlight_style(theme::selected_row())
    .highlight_symbol("▸ ")
    .style(theme::base())
    .column_spacing(1)
    .block(results_block(app));

    if app.estimate_state.selected().is_none() {
        app.estimate_state.select(Some(0));
    }
    frame.render_stateful_widget(table, area, &mut app.estimate_state);
}

fn model_name(model: &Model) -> Line<'static> {
    Line::from(vec![
        Span::styled(model.short_label(), theme::title()),
        Span::styled(
            format!("  {}", model.provider.label()),
            Style::new().fg(theme::provider(model.provider)),
        ),
    ])
}

fn results_block(app: &App) -> Block<'static> {
    let title = if app.using_implicit_selection() {
        " Cost of this mix · all visible models "
    } else {
        " Cost of this mix · selected models "
    };
    Block::default()
        .title(Span::styled(title, theme::muted()))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::border()))
        .style(theme::base())
        .padding(Padding::horizontal(1))
}
