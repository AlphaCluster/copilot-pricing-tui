use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Padding, Paragraph, Row, Table};

use super::{theme, widgets};
use crate::app::App;
use crate::calc::{CompareMetric, interaction_cost, vs_baseline};
use crate::columns::{ColumnPreset, CompareCol};
use crate::data::{Model, TYPICAL_USAGE};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let models: Vec<crate::data::Model> = ranked(app).into_iter().cloned().collect();
    if models.is_empty() {
        frame.render_widget(
            Paragraph::new("No models to compare.\n\nOpen Catalog and press Space to pick models.")
                .style(theme::muted())
                .block(
                    Block::default()
                        .style(theme::base())
                        .padding(Padding::uniform(2)),
                ),
            area,
        );
        return;
    }

    let [legend, table, detail] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(9),
    ])
    .areas(area);

    app.compare_width = area.width;
    draw_legend(frame, app, legend, &models);
    draw_table(frame, app, table, &models);
    draw_detail(frame, app, detail, &models);
    if app.column_picker {
        draw_picker(frame, app, area);
    }
}

fn ranked(app: &App) -> Vec<&Model> {
    let mut models = app.compare_set();
    models.sort_by_key(|model| app.compare_metric.rank_key(app.metric_of(model)));
    models
}

fn draw_legend(frame: &mut Frame, app: &App, area: Rect, models: &[Model]) {
    let source = if app.using_implicit_selection() {
        format!("all {} visible models", models.len())
    } else {
        format!("{} selected", models.len())
    };
    let leader = models.first().map(|m| m.short_label()).unwrap_or_default();
    let leader_word = if app.compare_metric.higher_better() {
        "best"
    } else {
        "cheapest"
    };
    let line = Line::from(vec![
        Span::styled("  ranked by  ", theme::muted()),
        widgets::badge(app.compare_metric.label(), theme::accent()),
        Span::styled("   m metric    v ", theme::muted()),
        widgets::badge(app.column_preset.label(), theme::surface2()),
        Span::styled("    o columns    b extra benches    ", theme::muted()),
        Span::styled(source, theme::muted()),
        Span::styled(format!("    {leader_word}: "), theme::muted()),
        Span::styled(
            leader,
            Style::new().fg(theme::green()).add_modifier(Modifier::BOLD),
        ),
        if app.auto_discount {
            Span::styled("    auto −10% on", Style::new().fg(theme::teal()))
        } else {
            Span::raw("")
        },
    ]);
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

fn draw_table(frame: &mut Frame, app: &mut App, area: Rect, models: &[Model]) {
    let max = models
        .iter()
        .map(|m| app.metric_of(m))
        .filter(|v| v.is_finite())
        .fold(0.0_f64, f64::max)
        .max(1e-9);
    let baseline = models
        .first()
        .map(|m| interaction_cost(m, TYPICAL_USAGE, app.auto_discount))
        .unwrap_or(crate::calc::Cost::from_usd(0.0));
    let cols = app.compare_columns(area.width);
    let extra_fixed: u16 = cols
        .iter()
        .map(|col| {
            if matches!(col, CompareCol::RankBar) {
                0
            } else {
                col.min_width() + 1
            }
        })
        .sum();
    let bar_w = (area.width.saturating_sub(28 + extra_fixed) as usize).clamp(8, 28);

    let mut headers = vec![Cell::from("#"), Cell::from("Model")];
    headers.extend(cols.iter().map(|col| {
        if *col == CompareCol::RankBar {
            Cell::from(app.compare_metric.label())
        } else {
            Cell::from(col.header())
        }
    }));
    let header =
        Row::new(headers).style(Style::new().fg(theme::dim()).add_modifier(Modifier::BOLD));

    let rows = models.iter().enumerate().map(|(i, model)| {
        let typical = interaction_cost(model, TYPICAL_USAGE, app.auto_discount);
        let metric = app.metric_of(model);
        let bench = app.catalog.bench_for(model);
        let rel = if app.compare_metric.higher_better() {
            let best = models
                .iter()
                .map(|m| app.metric_of(m))
                .find(|v| v.is_finite())
                .unwrap_or(metric);
            if metric.is_finite() && best > 0.0 {
                metric / best
            } else {
                f64::NAN
            }
        } else {
            vs_baseline(typical, baseline)
        };
        let mut cells = vec![
            Cell::from(Span::styled(format!("{}", i + 1), theme::muted())),
            Cell::from(Line::from({
                let mut spans = vec![
                    Span::styled(model.short_label(), theme::title()),
                    Span::styled(
                        format!("  {}", model.provider.label()),
                        Style::new().fg(theme::provider(model.provider)),
                    ),
                ];
                if model.is_promo() {
                    spans.push(Span::raw(" "));
                    spans.push(widgets::promo_badge());
                }
                spans
            })),
        ];
        for col in &cols {
            cells.push(col_cell(
                *col, model, typical, metric, rel, bench, app, max, bar_w,
            ));
        }
        Row::new(cells).style(theme::base())
    });

    let mut widths = vec![Constraint::Length(3), Constraint::Fill(2)];
    widths.extend(cols.iter().map(|col| col.constraint()));

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

    let selected = app.compare_state.selected().unwrap_or(0);
    if selected >= models.len() {
        app.compare_state
            .select(Some(models.len().saturating_sub(1)));
    } else if app.compare_state.selected().is_none() {
        app.compare_state.select(Some(0));
    }
    frame.render_stateful_widget(table, area, &mut app.compare_state);
}

#[allow(clippy::too_many_arguments)]
fn col_cell(
    col: CompareCol,
    model: &Model,
    typical: crate::calc::Cost,
    metric: f64,
    rel: f64,
    bench: Option<&crate::data::BenchScore>,
    app: &App,
    max: f64,
    bar_w: usize,
) -> Cell<'static> {
    match col {
        CompareCol::Input => Cell::from(widgets::money(model.input)),
        CompareCol::Cached => Cell::from(widgets::money(model.cached)),
        CompareCol::Write => Cell::from(widgets::money_opt(model.cache_write)),
        CompareCol::Output => Cell::from(widgets::money(model.output)),
        CompareCol::Typical => Cell::from(Span::styled(
            format!("{} cr", widgets::credits(typical.credits)),
            Style::new().fg(theme::fg()),
        )),
        CompareCol::Swe => score_cell(bench.and_then(|s| s.swe_verified)),
        CompareCol::Terminal => score_cell(bench.and_then(|s| s.terminal_bench)),
        CompareCol::LiveCode => score_cell(bench.and_then(|s| s.livecodebench)),
        CompareCol::VsLead => {
            let rel_label = if !rel.is_finite() {
                "—".into()
            } else if (rel - 1.0).abs() < 0.005 {
                if app.compare_metric.higher_better() {
                    "best".into()
                } else {
                    "cheapest".into()
                }
            } else {
                format!("{rel:.2}×")
            };
            Cell::from(Span::styled(
                rel_label,
                if rel.is_finite() && (rel - 1.0).abs() < 0.005 {
                    Style::new().fg(theme::green())
                } else if rel.is_finite() {
                    Style::new().fg(theme::heat(((rel - 1.0).abs() / 8.0).clamp(0.0, 1.0)))
                } else {
                    theme::muted()
                },
            ))
        }
        CompareCol::RankBar => {
            let heat_ratio = if !metric.is_finite() {
                0.0
            } else if app.compare_metric.higher_better() {
                1.0 - (metric / max).clamp(0.0, 1.0)
            } else {
                (metric / max).clamp(0.0, 1.0)
            };
            let mut bar_spans = vec![Span::styled(
                format!(
                    "{:>8}  ",
                    metric_label(app.compare_metric, model, typical, metric)
                ),
                Style::new().fg(theme::heat(heat_ratio)),
            )];
            if metric.is_finite() {
                bar_spans.extend(widgets::bar_line(metric / max, bar_w).spans);
            }
            Cell::from(Line::from(bar_spans))
        }
    }
}

fn score_cell(value: Option<f64>) -> Cell<'static> {
    Cell::from(Span::styled(
        value
            .map(|v| format!("{v:.1}%"))
            .unwrap_or_else(|| "—".into()),
        Style::new().fg(if value.is_some() {
            theme::teal()
        } else {
            theme::dim()
        }),
    ))
}

fn draw_picker(frame: &mut Frame, app: &App, area: Rect) {
    let height = area.height.saturating_sub(1).clamp(12, 18);
    let [popup] = Layout::horizontal([Constraint::Length(42)])
        .flex(Flex::End)
        .areas(area);
    let [popup] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Start)
        .areas(popup);
    frame.render_widget(Clear, popup);

    let visible = app.compare_columns(app.compare_width);
    let wanted = if app.column_preset == ColumnPreset::Custom {
        app.custom_columns
    } else {
        crate::columns::ColSet::from_cols(&visible)
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Layout  ", theme::muted()),
            widgets::badge(app.column_preset.label(), theme::accent()),
            if app.column_preset == ColumnPreset::Auto {
                Span::styled(format!("  {} cols", app.compare_width), theme::muted())
            } else {
                Span::raw("")
            },
        ]),
        Line::from(Span::styled(
            "v cycle   b extra benches   space toggle",
            theme::muted(),
        )),
        Line::raw(""),
    ];
    for (i, col) in CompareCol::ALL.iter().enumerate() {
        let on = wanted.contains(*col);
        let fitted = visible.contains(col);
        let cursor = i == app.picker_cursor;
        let mark = if on { "●" } else { "○" };
        let prefix = if cursor { "▸ " } else { "  " };
        let style = if cursor {
            theme::selected_row()
        } else if on {
            Style::new().fg(theme::fg())
        } else {
            theme::muted()
        };
        let extra = if on && !fitted { "  (narrow)" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(prefix, theme::muted()),
            Span::styled(format!("{mark}  {}{extra}", col.picker_label()), style),
        ]));
    }

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(Span::styled(" Columns ", theme::title()))
                .borders(Borders::ALL)
                .border_style(Style::new().fg(theme::accent()))
                .style(Style::new().bg(theme::surface()).fg(theme::fg()))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        popup,
    );
}

fn metric_label(
    metric: CompareMetric,
    model: &Model,
    typical: crate::calc::Cost,
    value: f64,
) -> String {
    match metric {
        CompareMetric::Typical => format!("{} cr", widgets::credits(typical.credits)),
        CompareMetric::CacheWrite if model.cache_write.is_none() => "—".into(),
        CompareMetric::SweVerified | CompareMetric::TerminalBench | CompareMetric::LiveCode => {
            if value.is_finite() {
                format!("{value:.1}%")
            } else {
                "—".into()
            }
        }
        _ => widgets::money(value),
    }
}

fn draw_detail(frame: &mut Frame, app: &App, area: Rect, models: &[Model]) {
    let idx = app
        .compare_state
        .selected()
        .unwrap_or(0)
        .min(models.len() - 1);
    let model = &models[idx];
    let typical = interaction_cost(model, TYPICAL_USAGE, app.auto_discount);
    let cheapest = interaction_cost(&models[0], TYPICAL_USAGE, app.auto_discount);
    let plan = app.plan();
    let fits = crate::calc::interactions_per_plan(typical, plan);
    let rel = vs_baseline(typical, cheapest);

    let max_in = models.iter().map(|m| m.input).fold(0.0, f64::max).max(1e-9);
    let max_out = models
        .iter()
        .map(|m| m.output)
        .fold(0.0, f64::max)
        .max(1e-9);
    let max_cached = models
        .iter()
        .map(|m| m.cached)
        .fold(0.0, f64::max)
        .max(1e-9);
    let max_write = models
        .iter()
        .filter_map(|m| m.cache_write)
        .fold(0.0, f64::max)
        .max(1e-9);

    let rate_line = |label: &str, value: f64, max: f64, missing: bool| {
        let mut spans = vec![Span::styled(format!("  {label:<12}"), theme::muted())];
        if missing {
            spans.push(Span::styled("—", theme::muted()));
        } else {
            spans.push(Span::styled(
                format!("{:>7}  ", widgets::money(value)),
                theme::title(),
            ));
            spans.extend(widgets::bar_line(value / max, 24).spans);
        }
        Line::from(spans)
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(format!("  {}", model.label()), theme::title()),
            if model.is_promo() {
                Span::raw(" ")
            } else {
                Span::raw("")
            },
            if model.is_promo() {
                widgets::promo_badge()
            } else {
                Span::raw("")
            },
            Span::styled(
                format!(
                    "   {} coding-chat turns / mo on {}",
                    widgets::count(fits),
                    plan.name
                ),
                theme::muted(),
            ),
            Span::styled(
                format!("   {}× the cheapest selected", format_rel(rel)),
                Style::new().fg(theme::heat(((rel - 1.0) / 8.0).clamp(0.0, 1.0))),
            ),
        ]),
        bench_line(app, model),
        rate_line("input", model.input, max_in, false),
        rate_line("cached", model.cached, max_cached, false),
        rate_line(
            "cache write",
            model.cache_write.unwrap_or(0.0),
            max_write,
            model.cache_write.is_none(),
        ),
        rate_line("output", model.output, max_out, false),
    ];

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::new().fg(theme::border()))
                .style(theme::base()),
        ),
        area,
    );
}

fn bench_line(app: &App, model: &Model) -> Line<'static> {
    let Some(score) = app.catalog.bench_for(model) else {
        return Line::from(Span::styled(
            "  SWE benches   —  not on the Vals leaderboard yet",
            theme::muted(),
        ));
    };
    Line::from(vec![
        Span::styled("  SWE-V ", theme::muted()),
        Span::styled(fmt_score(score.swe_verified), theme::title()),
        Span::styled("    TB 2.1 ", theme::muted()),
        Span::styled(fmt_score(score.terminal_bench), theme::title()),
        Span::styled("    LiveCode ", theme::muted()),
        Span::styled(fmt_score(score.livecodebench), theme::title()),
        Span::styled("    vals.ai  mini-SWE-agent", theme::muted()),
    ])
}

fn fmt_score(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{v:>5.1}%"),
        None => "    —".into(),
    }
}

fn format_rel(rel: f64) -> String {
    if (rel - 1.0).abs() < 0.005 {
        "1.00".into()
    } else {
        format!("{rel:.2}")
    }
}
