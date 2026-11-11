use std::io;
use std::time::Duration;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::TableState;

use crate::calc::{CompareMetric, interaction_cost};
use crate::columns::{ColSet, ColumnPreset, CompareCol, preset_columns};
use crate::data::{
    Catalog, Category, Model, Plan, Provider, SCENARIOS, Scenario, TYPICAL_USAGE, Usage,
};
use crate::ui;
use crate::ui::theme::ThemeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Catalog,
    Compare,
    Estimate,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Catalog, Tab::Compare, Tab::Estimate];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Catalog => "Catalog",
            Tab::Compare => "Compare",
            Tab::Estimate => "Estimate",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Tab::Catalog => Tab::Compare,
            Tab::Compare => Tab::Estimate,
            Tab::Estimate => Tab::Catalog,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Tab::Catalog => Tab::Estimate,
            Tab::Compare => Tab::Catalog,
            Tab::Estimate => Tab::Compare,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Provider,
    Category,
    Input,
    Cached,
    Output,
    Typical,
    Swe,
}

impl SortKey {
    pub const ALL: [SortKey; 8] = [
        SortKey::Name,
        SortKey::Provider,
        SortKey::Category,
        SortKey::Input,
        SortKey::Cached,
        SortKey::Output,
        SortKey::Typical,
        SortKey::Swe,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SortKey::Name => "name",
            SortKey::Provider => "provider",
            SortKey::Category => "category",
            SortKey::Input => "input $",
            SortKey::Cached => "cached $",
            SortKey::Output => "output $",
            SortKey::Typical => "typical",
            SortKey::Swe => "SWE-bench",
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|k| *k == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstField {
    Input,
    Cached,
    CacheWrite,
    Output,
}

impl EstField {
    pub const ALL: [EstField; 4] = [
        EstField::Input,
        EstField::Cached,
        EstField::CacheWrite,
        EstField::Output,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EstField::Input => "Input tokens",
            EstField::Cached => "Cached input",
            EstField::CacheWrite => "Cache write",
            EstField::Output => "Output tokens",
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let i = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

pub struct App {
    pub catalog: Catalog,
    pub theme_id: ThemeId,
    pub tab: Tab,
    pub plan_idx: usize,
    pub selected: Vec<String>,
    pub show_help: bool,
    pub auto_discount: bool,
    pub should_quit: bool,
    pub status: Option<String>,
    pub status_error: bool,
    pub pending_refresh: bool,

    pub search: String,
    pub searching: bool,
    pub provider_filter: Option<Provider>,
    pub category_filter: Option<Category>,
    pub sort: SortKey,
    pub sort_desc: bool,
    pub catalog_state: TableState,

    pub compare_metric: CompareMetric,
    pub compare_state: TableState,
    pub column_preset: ColumnPreset,
    pub custom_columns: ColSet,
    pub column_picker: bool,
    pub picker_cursor: usize,
    pub compare_width: u16,

    pub scenario_idx: usize,
    pub usage: Usage,
    pub est_field: EstField,
    pub estimate_state: TableState,
}

impl App {
    pub fn new() -> Self {
        let mut catalog_state = TableState::default();
        catalog_state.select(Some(0));
        let catalog = Catalog::load();
        let session = restore_session();
        let selected = session.clone().into_existing(&catalog);
        Self {
            catalog,
            theme_id: session.theme,
            tab: Tab::Catalog,
            plan_idx: 1, // Pro+ is the most common "I want to compare models" plan
            selected,
            show_help: false,
            auto_discount: false,
            should_quit: false,
            status: None,
            status_error: false,
            pending_refresh: false,
            search: String::new(),
            searching: false,
            provider_filter: None,
            category_filter: None,
            sort: SortKey::Typical,
            sort_desc: false,
            catalog_state,
            compare_metric: CompareMetric::Typical,
            compare_state: TableState::default(),
            column_preset: ColumnPreset::Auto,
            custom_columns: ColSet::empty(),
            column_picker: false,
            picker_cursor: 0,
            compare_width: 120,
            scenario_idx: 1,
            usage: SCENARIOS[1].usage(),
            est_field: EstField::Input,
            estimate_state: TableState::default(),
        }
    }

    pub fn plan(&self) -> &Plan {
        let idx = self
            .plan_idx
            .min(self.catalog.plans.len().saturating_sub(1));
        &self.catalog.plans[idx]
    }

    pub fn scenario(&self) -> Scenario {
        SCENARIOS[self.scenario_idx]
    }

    pub fn is_selected(&self, id: &str) -> bool {
        self.selected.iter().any(|s| s == id)
    }

    fn persist_selection(&self) {
        if cfg!(test) && std::env::var_os("COPILOT_PRICING_SESSION").is_none() {
            return;
        }
        let session = crate::session::Session {
            selected: self.selected.clone(),
            theme: self.theme_id,
        };
        let _ = session.save();
    }

    fn cycle_theme(&mut self, reverse: bool) {
        self.theme_id = if reverse {
            self.theme_id.prev()
        } else {
            self.theme_id.next()
        };
        crate::ui::theme::set_current(self.theme_id);
        self.persist_selection();
        self.status = Some(format!("Theme: {}", self.theme_id.label()));
        self.status_error = false;
    }

    fn clear_selection(&mut self) {
        if self.selected.is_empty() {
            self.status = Some("No comparison to clear".into());
            self.status_error = false;
            return;
        }
        let n = self.selected.len();
        self.selected.clear();
        self.persist_selection();
        self.status = Some(format!(
            "Cleared {n} model{}",
            if n == 1 { "" } else { "s" }
        ));
        self.status_error = false;
    }

    pub fn selected_models(&self) -> Vec<&Model> {
        self.selected
            .iter()
            .filter_map(|id| self.catalog.model_by_id(id))
            .collect()
    }

    /// Models the compare/estimate views operate on.
    /// Selected models if any, otherwise every currently visible catalog row.
    pub fn compare_set(&self) -> Vec<&Model> {
        let selected = self.selected_models();
        if selected.is_empty() {
            self.visible_models()
        } else {
            selected
        }
    }

    pub fn using_implicit_selection(&self) -> bool {
        self.selected.is_empty()
    }

    pub fn compare_columns(&self, width: u16) -> Vec<CompareCol> {
        match self.column_preset {
            ColumnPreset::Custom => crate::columns::fit_columns(self.custom_columns.cols(), width),
            other => preset_columns(other, width),
        }
    }

    pub fn cycle_column_preset(&mut self) {
        self.column_preset = self.column_preset.next();
        self.column_picker = false;
    }

    pub fn toggle_picker_column(&mut self) {
        let col = CompareCol::ALL[self.picker_cursor.min(CompareCol::ALL.len() - 1)];
        if self.column_preset != ColumnPreset::Custom {
            self.custom_columns = ColSet::from_cols(&self.compare_columns(self.compare_width));
            self.column_preset = ColumnPreset::Custom;
        }
        self.custom_columns.toggle(col);
        if self.custom_columns.cols().is_empty() {
            self.custom_columns.insert(CompareCol::RankBar);
        }
    }

    pub fn toggle_extra_benches(&mut self) {
        if self.column_preset != ColumnPreset::Custom {
            self.custom_columns = ColSet::from_cols(&self.compare_columns(self.compare_width));
            self.column_preset = ColumnPreset::Custom;
        }
        let show = !self.custom_columns.contains(CompareCol::Terminal)
            || !self.custom_columns.contains(CompareCol::LiveCode);
        self.custom_columns.set(CompareCol::Terminal, show);
        self.custom_columns.set(CompareCol::LiveCode, show);
        if show {
            self.custom_columns.insert(CompareCol::Swe);
        }
    }

    pub fn metric_of(&self, model: &Model) -> f64 {
        crate::calc::metric_value(
            model,
            TYPICAL_USAGE,
            self.auto_discount,
            self.compare_metric,
            self.catalog.bench_for(model),
        )
    }

    pub fn visible_models(&self) -> Vec<&Model> {
        let mut rows: Vec<&Model> = self
            .catalog
            .models
            .iter()
            .filter(|model| self.matches_filters(model))
            .collect();
        rows.sort_by(|a, b| {
            let ord = match self.sort {
                SortKey::Name => a.label().to_lowercase().cmp(&b.label().to_lowercase()),
                SortKey::Provider => a
                    .provider
                    .label()
                    .cmp(b.provider.label())
                    .then_with(|| a.name.cmp(&b.name)),
                SortKey::Category => (a.category as u8)
                    .cmp(&(b.category as u8))
                    .then_with(|| a.name.cmp(&b.name)),
                SortKey::Input => cmp_f64(a.input, b.input),
                SortKey::Cached => cmp_f64(a.cached, b.cached),
                SortKey::Output => cmp_f64(a.output, b.output),
                SortKey::Typical => cmp_f64(
                    interaction_cost(a, TYPICAL_USAGE, false).usd,
                    interaction_cost(b, TYPICAL_USAGE, false).usd,
                ),
                SortKey::Swe => cmp_f64(
                    self.catalog
                        .bench_for(b)
                        .and_then(|score| score.primary())
                        .unwrap_or(-1.0),
                    self.catalog
                        .bench_for(a)
                        .and_then(|score| score.primary())
                        .unwrap_or(-1.0),
                ),
            };
            if self.sort_desc { ord.reverse() } else { ord }
        });
        rows
    }

    fn matches_filters(&self, model: &Model) -> bool {
        if let Some(provider) = self.provider_filter
            && model.provider != provider
        {
            return false;
        }
        if let Some(category) = self.category_filter
            && model.category != category
        {
            return false;
        }
        if self.search.is_empty() {
            return true;
        }
        let q = self.search.to_lowercase();
        model.name.to_lowercase().contains(&q)
            || model.variant.to_lowercase().contains(&q)
            || model.provider.label().to_lowercase().contains(&q)
            || model.category.label().to_lowercase().contains(&q)
            || model
                .notes
                .as_deref()
                .is_some_and(|note| note.to_lowercase().contains(&q))
    }

    pub fn catalog_cursor(&self) -> usize {
        self.catalog_state.selected().unwrap_or(0)
    }

    fn clamp_catalog_cursor(&mut self) {
        let len = self.visible_models().len();
        if len == 0 {
            self.catalog_state.select(None);
            return;
        }
        let idx = self.catalog_cursor().min(len - 1);
        self.catalog_state.select(Some(idx));
    }

    fn move_catalog(&mut self, delta: isize) {
        let len = self.visible_models().len();
        if len == 0 {
            return;
        }
        let cur = self.catalog_cursor() as isize;
        let next = (cur + delta).rem_euclid(len as isize) as usize;
        self.catalog_state.select(Some(next));
    }

    fn move_compare(&mut self, delta: isize) {
        let len = self.compare_set().len();
        if len == 0 {
            self.compare_state.select(None);
            return;
        }
        let cur = self.compare_state.selected().unwrap_or(0) as isize;
        let next = (cur + delta).clamp(0, len as isize - 1) as usize;
        self.compare_state.select(Some(next));
    }

    fn move_estimate(&mut self, delta: isize) {
        let len = self.compare_set().len();
        if len == 0 {
            self.estimate_state.select(None);
            return;
        }
        let cur = self.estimate_state.selected().unwrap_or(0) as isize;
        let next = (cur + delta).clamp(0, len as isize - 1) as usize;
        self.estimate_state.select(Some(next));
    }

    fn toggle_current(&mut self) {
        let Some(id) = self
            .visible_models()
            .get(self.catalog_cursor())
            .map(|model| model.id.clone())
        else {
            return;
        };
        if let Some(pos) = self.selected.iter().position(|existing| existing == &id) {
            self.selected.remove(pos);
        } else {
            self.selected.push(id);
        }
        self.persist_selection();
    }

    fn select_all_visible(&mut self) {
        let ids: Vec<String> = self
            .visible_models()
            .into_iter()
            .map(|model| model.id.clone())
            .collect();
        for id in ids {
            if !self.is_selected(&id) {
                self.selected.push(id);
            }
        }
        self.persist_selection();
    }

    fn cycle_provider(&mut self, reverse: bool) {
        let providers = self.catalog.providers();
        self.provider_filter = cycle_option(self.provider_filter, &providers, reverse);
        self.clamp_catalog_cursor();
    }

    fn cycle_category(&mut self, reverse: bool) {
        self.category_filter = cycle_option(self.category_filter, &Category::ALL, reverse);
        self.clamp_catalog_cursor();
    }

    fn cycle_plan(&mut self, reverse: bool) {
        let n = self.catalog.plans.len();
        if n == 0 {
            return;
        }
        self.plan_idx = if reverse {
            (self.plan_idx + n - 1) % n
        } else {
            (self.plan_idx + 1) % n
        };
    }

    fn apply_catalog(&mut self, catalog: Catalog) {
        let keep: Vec<String> = self
            .selected
            .iter()
            .filter(|id| catalog.model_by_id(id).is_some())
            .cloned()
            .collect();
        self.selected = keep;
        self.persist_selection();
        if self.plan_idx >= catalog.plans.len() {
            self.plan_idx = catalog.plans.len().saturating_sub(1);
        }
        self.catalog = catalog;
        if let Some(other) = self.provider_filter
            && !self.catalog.providers().contains(&other)
        {
            self.provider_filter = None;
        }
        self.clamp_catalog_cursor();
    }

    fn request_refresh(&mut self) {
        self.pending_refresh = true;
        self.status = Some("Fetching GitHub Copilot pricing…".into());
        self.status_error = false;
    }

    fn cycle_scenario(&mut self, reverse: bool) {
        let n = SCENARIOS.len();
        self.scenario_idx = if reverse {
            (self.scenario_idx + n - 1) % n
        } else {
            (self.scenario_idx + 1) % n
        };
        self.usage = SCENARIOS[self.scenario_idx].usage();
    }

    fn adjust_field(&mut self, grow: bool, coarse: bool) {
        let step = match (self.est_field, coarse) {
            (EstField::Input, false) => 1_000,
            (EstField::Input, true) => 10_000,
            (EstField::Cached, false) => 500,
            (EstField::Cached, true) => 5_000,
            (EstField::CacheWrite, false) => 250,
            (EstField::CacheWrite, true) => 2_500,
            (EstField::Output, false) => 250,
            (EstField::Output, true) => 2_500,
        };
        let slot = match self.est_field {
            EstField::Input => &mut self.usage.input,
            EstField::Cached => &mut self.usage.cached,
            EstField::CacheWrite => &mut self.usage.cache_write,
            EstField::Output => &mut self.usage.output,
        };
        if grow {
            *slot = slot.saturating_add(step).min(2_000_000);
        } else {
            *slot = slot.saturating_sub(step);
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        if self.status.is_some() && !self.pending_refresh {
            self.status = None;
            self.status_error = false;
        }
        if self.show_help {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter => {
                    self.show_help = false;
                }
                _ => {}
            }
            return;
        }

        if self.column_picker {
            match key.code {
                KeyCode::Esc | KeyCode::Char('o') | KeyCode::Char('q') => {
                    self.column_picker = false;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.picker_cursor = (self.picker_cursor + 1) % CompareCol::ALL.len();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.picker_cursor =
                        (self.picker_cursor + CompareCol::ALL.len() - 1) % CompareCol::ALL.len();
                }
                KeyCode::Char(' ') | KeyCode::Enter => self.toggle_picker_column(),
                KeyCode::Char('v') => self.cycle_column_preset(),
                KeyCode::Char('b') => self.toggle_extra_benches(),
                _ => {}
            }
            return;
        }

        if self.searching {
            match key.code {
                KeyCode::Esc => {
                    self.searching = false;
                    self.search.clear();
                    self.clamp_catalog_cursor();
                }
                KeyCode::Enter => self.searching = false,
                KeyCode::Backspace => {
                    self.search.pop();
                    self.clamp_catalog_cursor();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.search.clear();
                    self.clamp_catalog_cursor();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.search.push(c);
                    self.clamp_catalog_cursor();
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('1') => self.tab = Tab::Catalog,
            KeyCode::Char('2') => self.tab = Tab::Compare,
            KeyCode::Char('3') => self.tab = Tab::Estimate,
            KeyCode::Tab if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.tab = self.tab.next();
            }
            KeyCode::BackTab => self.tab = self.tab.prev(),
            KeyCode::Char('p') => self.cycle_plan(key.modifiers.contains(KeyModifiers::SHIFT)),
            KeyCode::Char('d') => self.auto_discount = !self.auto_discount,
            KeyCode::Char('t') => self.cycle_theme(false),
            KeyCode::Char('T') => self.cycle_theme(true),
            KeyCode::Char('x') | KeyCode::Char('X') => self.clear_selection(),
            KeyCode::Char('u') => self.request_refresh(),
            KeyCode::Char('/') => {
                self.tab = Tab::Catalog;
                self.searching = true;
            }
            other => match self.tab {
                Tab::Catalog => self.on_catalog_key(other, key.modifiers),
                Tab::Compare => self.on_compare_key(other, key.modifiers),
                Tab::Estimate => self.on_estimate_key(other, key.modifiers),
            },
        }
    }

    fn on_catalog_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        match code {
            KeyCode::Down | KeyCode::Char('j') => self.move_catalog(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_catalog(-1),
            KeyCode::PageDown => self.move_catalog(12),
            KeyCode::PageUp => self.move_catalog(-12),
            KeyCode::Home | KeyCode::Char('g') => {
                if !self.visible_models().is_empty() {
                    self.catalog_state.select(Some(0));
                }
            }
            KeyCode::End | KeyCode::Char('G') => {
                let len = self.visible_models().len();
                if len > 0 {
                    self.catalog_state.select(Some(len - 1));
                }
            }
            KeyCode::Char(' ') => self.toggle_current(),
            KeyCode::Enter => {
                if self.selected.is_empty() {
                    self.toggle_current();
                }
                self.tab = Tab::Compare;
            }
            KeyCode::Char('a') => self.select_all_visible(),
            KeyCode::Char('f') => self.cycle_provider(mods.contains(KeyModifiers::SHIFT)),
            KeyCode::Char('c') => self.cycle_category(mods.contains(KeyModifiers::SHIFT)),
            KeyCode::Char('s') => {
                if mods.contains(KeyModifiers::SHIFT) {
                    self.sort_desc = !self.sort_desc;
                } else {
                    self.sort = self.sort.next();
                }
                self.clamp_catalog_cursor();
            }
            _ => {}
        }
    }

    fn on_compare_key(&mut self, code: KeyCode, _mods: KeyModifiers) {
        match code {
            KeyCode::Down | KeyCode::Char('j') => self.move_compare(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_compare(-1),
            KeyCode::Char('m') | KeyCode::Right | KeyCode::Char('l') => {
                self.compare_metric = self.compare_metric.next();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.compare_metric = self.compare_metric.prev();
            }
            KeyCode::Char('v') => self.cycle_column_preset(),
            KeyCode::Char('b') => self.toggle_extra_benches(),
            KeyCode::Char('o') => {
                self.column_picker = true;
                self.picker_cursor = 0;
            }
            KeyCode::Char(' ') => {
                // jump back so the user can change the set
                self.tab = Tab::Catalog;
            }
            _ => {}
        }
    }

    fn on_estimate_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        match code {
            KeyCode::Down | KeyCode::Char('j') => self.move_estimate(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_estimate(-1),
            KeyCode::Char('n') => self.cycle_scenario(mods.contains(KeyModifiers::SHIFT)),
            KeyCode::Char(',') | KeyCode::Char('[') => self.est_field = self.est_field.prev(),
            KeyCode::Char('.') | KeyCode::Char(']') => self.est_field = self.est_field.next(),
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('-') => {
                self.adjust_field(false, mods.contains(KeyModifiers::SHIFT));
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('=') | KeyCode::Char('+') => {
                self.adjust_field(true, mods.contains(KeyModifiers::SHIFT));
            }
            KeyCode::Char('H') => self.adjust_field(false, true),
            KeyCode::Char('L') => self.adjust_field(true, true),
            _ => {}
        }
    }

    fn run_loop(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        if self.plan_idx >= self.catalog.plans.len() {
            self.plan_idx = self.catalog.plans.len().saturating_sub(1);
        }
        while !self.should_quit {
            terminal.draw(|frame| ui::draw(frame, &mut self))?;
            if self.pending_refresh {
                self.pending_refresh = false;
                match crate::fetch::refresh_and_save() {
                    Ok((catalog, path)) => {
                        let n = catalog.models.len();
                        let scored = catalog.scored_count();
                        self.apply_catalog(catalog);
                        self.status = Some(format!(
                            "Updated {n} models, {scored} with SWE scores → {}",
                            path.display()
                        ));
                        self.status_error = false;
                    }
                    Err(err) => {
                        self.status = Some(format!("Update failed: {err}"));
                        self.status_error = true;
                    }
                }
                continue;
            }
            if event::poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key(key),
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
        }
        self.persist_selection();
        Ok(())
    }
}

fn restore_session() -> crate::session::Session {
    if cfg!(test) && std::env::var_os("COPILOT_PRICING_SESSION").is_none() {
        return crate::session::Session::default();
    }
    crate::session::Session::load()
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = App::new().run_loop(&mut terminal);
    ratatui::restore();
    result
}

fn cmp_f64(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}

fn cycle_option<T: Copy + PartialEq>(current: Option<T>, items: &[T], reverse: bool) -> Option<T> {
    match current {
        None if reverse => items.last().copied(),
        None => items.first().copied(),
        Some(value) => {
            let i = items.iter().position(|item| *item == value);
            match i {
                Some(i) if reverse && i == 0 => None,
                Some(i) if !reverse && i + 1 == items.len() => None,
                Some(i) if reverse => Some(items[i - 1]),
                Some(i) => Some(items[i + 1]),
                None => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_filters_models() {
        let mut app = App::new();
        app.search = "grok".into();
        let visible = app.visible_models();
        assert!(visible.iter().all(|m| m.provider == Provider::Xai));
        assert_eq!(visible.len(), 6);
    }

    #[test]
    fn provider_filter_and_selection() {
        let mut app = App::new();
        app.provider_filter = Some(Provider::Microsoft);
        let visible = app.visible_models();
        assert_eq!(visible.len(), 1);
        app.catalog_state.select(Some(0));
        app.toggle_current();
        assert_eq!(app.selected.len(), 1);
        app.toggle_current();
        assert!(app.selected.is_empty());
    }

    #[test]
    fn estimate_field_adjust_clamps() {
        let mut app = App::new();
        app.usage.input = 500;
        app.est_field = EstField::Input;
        app.adjust_field(false, false);
        assert_eq!(app.usage.input, 0);
        app.usage.input = 1_999_000;
        app.adjust_field(true, true);
        assert_eq!(app.usage.input, 2_000_000);
    }

    #[test]
    fn compare_layout_cycle_and_extra_benches() {
        let mut app = App::new();
        assert_eq!(app.column_preset, ColumnPreset::Auto);
        let auto = app.compare_columns(90);
        assert!(auto.contains(&CompareCol::Swe));
        assert!(!auto.contains(&CompareCol::LiveCode));

        app.cycle_column_preset();
        assert_eq!(app.column_preset, ColumnPreset::Pricing);
        app.cycle_column_preset();
        assert_eq!(app.column_preset, ColumnPreset::Benches);
        let benches = app.compare_columns(160);
        assert!(benches.contains(&CompareCol::Terminal));
        assert!(benches.contains(&CompareCol::LiveCode));

        app.column_preset = ColumnPreset::Auto;
        app.compare_width = 90;
        app.toggle_extra_benches();
        assert_eq!(app.column_preset, ColumnPreset::Custom);
        assert!(app.custom_columns.contains(CompareCol::Terminal));
        assert!(app.custom_columns.contains(CompareCol::LiveCode));
        app.toggle_extra_benches();
        assert!(!app.custom_columns.contains(CompareCol::Terminal));
        assert!(!app.custom_columns.contains(CompareCol::LiveCode));
    }

    #[test]
    fn clear_selection_empties_compare_set() {
        let mut app = App::new();
        app.selected = vec!["gpt-5.4-nano".into(), "claude-opus-5".into()];
        app.clear_selection();
        assert!(app.selected.is_empty());
        app.clear_selection();
        assert_eq!(app.status.as_deref(), Some("No comparison to clear"));
    }

    #[test]
    fn theme_cycles_through_the_set() {
        let mut app = App::new();
        assert_eq!(app.theme_id, ThemeId::GitHub);
        app.cycle_theme(false);
        assert_eq!(app.theme_id, ThemeId::Dracula);
        app.cycle_theme(true);
        assert_eq!(app.theme_id, ThemeId::GitHub);
    }
}
