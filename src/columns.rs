//! Compare-table column presets and hide/show state.

use ratatui::layout::Constraint;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareCol {
    Input,
    Cached,
    Write,
    Output,
    Typical,
    Swe,
    Terminal,
    LiveCode,
    VsLead,
    RankBar,
}

impl CompareCol {
    pub const ALL: [CompareCol; 10] = [
        CompareCol::Input,
        CompareCol::Cached,
        CompareCol::Write,
        CompareCol::Output,
        CompareCol::Typical,
        CompareCol::Swe,
        CompareCol::Terminal,
        CompareCol::LiveCode,
        CompareCol::VsLead,
        CompareCol::RankBar,
    ];

    pub fn header(self) -> &'static str {
        match self {
            CompareCol::Input => "In",
            CompareCol::Cached => "Cache",
            CompareCol::Write => "Write",
            CompareCol::Output => "Out",
            CompareCol::Typical => "Typical",
            CompareCol::Swe => "SWE-V",
            CompareCol::Terminal => "TB 2.1",
            CompareCol::LiveCode => "LiveCode",
            CompareCol::VsLead => "vs lead",
            CompareCol::RankBar => "ranked",
        }
    }

    pub fn picker_label(self) -> &'static str {
        match self {
            CompareCol::Input => "Input $",
            CompareCol::Cached => "Cached $",
            CompareCol::Write => "Cache write $",
            CompareCol::Output => "Output $",
            CompareCol::Typical => "Typical chat cost",
            CompareCol::Swe => "SWE-bench Verified",
            CompareCol::Terminal => "Terminal-Bench 2.1",
            CompareCol::LiveCode => "LiveCodeBench",
            CompareCol::VsLead => "vs leader",
            CompareCol::RankBar => "Rank bar",
        }
    }

    pub fn constraint(self) -> Constraint {
        match self {
            CompareCol::Typical => Constraint::Length(10),
            CompareCol::VsLead => Constraint::Length(10),
            CompareCol::RankBar => Constraint::Fill(3),
            _ => Constraint::Length(8),
        }
    }

    pub fn min_width(self) -> u16 {
        match self {
            CompareCol::Typical | CompareCol::VsLead => 10,
            CompareCol::RankBar => 14,
            _ => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnPreset {
    Auto,
    Pricing,
    Benches,
    Full,
    Custom,
}

impl ColumnPreset {
    pub const CYCLE: [ColumnPreset; 4] = [
        ColumnPreset::Auto,
        ColumnPreset::Pricing,
        ColumnPreset::Benches,
        ColumnPreset::Full,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ColumnPreset::Auto => "auto",
            ColumnPreset::Pricing => "pricing",
            ColumnPreset::Benches => "benches",
            ColumnPreset::Full => "full",
            ColumnPreset::Custom => "custom",
        }
    }

    pub fn next(self) -> Self {
        match self {
            ColumnPreset::Auto => ColumnPreset::Pricing,
            ColumnPreset::Pricing => ColumnPreset::Benches,
            ColumnPreset::Benches => ColumnPreset::Full,
            ColumnPreset::Full | ColumnPreset::Custom => ColumnPreset::Auto,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColSet(u16);

impl ColSet {
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn from_cols(cols: &[CompareCol]) -> Self {
        let mut set = Self::empty();
        for col in cols {
            set.insert(*col);
        }
        set
    }

    pub fn contains(self, col: CompareCol) -> bool {
        self.0 & (1 << col as u16) != 0
    }

    pub fn insert(&mut self, col: CompareCol) {
        self.0 |= 1 << col as u16;
    }

    pub fn toggle(&mut self, col: CompareCol) {
        self.0 ^= 1 << col as u16;
    }

    pub fn set(&mut self, col: CompareCol, on: bool) {
        if on {
            self.insert(col);
        } else {
            self.0 &= !(1 << col as u16);
        }
    }

    pub fn cols(self) -> Vec<CompareCol> {
        CompareCol::ALL
            .into_iter()
            .filter(|col| self.contains(*col))
            .collect()
    }
}

pub fn preset_columns(preset: ColumnPreset, width: u16) -> Vec<CompareCol> {
    let cols = match preset {
        ColumnPreset::Auto => auto_columns(width),
        ColumnPreset::Pricing => vec![
            CompareCol::Input,
            CompareCol::Cached,
            CompareCol::Write,
            CompareCol::Output,
            CompareCol::Typical,
            CompareCol::VsLead,
            CompareCol::RankBar,
        ],
        ColumnPreset::Benches => vec![
            CompareCol::Typical,
            CompareCol::Swe,
            CompareCol::Terminal,
            CompareCol::LiveCode,
            CompareCol::VsLead,
            CompareCol::RankBar,
        ],
        ColumnPreset::Full | ColumnPreset::Custom => CompareCol::ALL.to_vec(),
    };
    fit_columns(cols, width)
}

pub fn auto_columns(width: u16) -> Vec<CompareCol> {
    let mut cols = vec![CompareCol::Typical, CompareCol::Swe, CompareCol::RankBar];
    if width >= 108 {
        cols.splice(0..0, [CompareCol::Input, CompareCol::Output]);
        cols.insert(cols.len() - 1, CompareCol::VsLead);
    }
    if width >= 132 {
        cols.splice(1..1, [CompareCol::Cached, CompareCol::Write]);
        let rank_at = cols
            .iter()
            .position(|c| *c == CompareCol::RankBar)
            .unwrap_or(cols.len());
        cols.splice(
            rank_at..rank_at,
            [CompareCol::Terminal, CompareCol::LiveCode],
        );
    }
    cols
}

/// Drop lowest-priority optional columns until the row fits.
pub fn fit_columns(cols: Vec<CompareCol>, width: u16) -> Vec<CompareCol> {
    let reserved = 28; // # + model + padding
    let budget = width.saturating_sub(reserved);
    let mut chosen = cols;
    while !chosen.is_empty() {
        let needed: u16 = chosen.iter().map(|c| c.min_width() + 1).sum();
        if needed <= budget {
            break;
        }
        let drop = chosen
            .iter()
            .rev()
            .find(|col| **col != CompareCol::RankBar && **col != CompareCol::Swe)
            .copied()
            .or_else(|| {
                chosen
                    .iter()
                    .rev()
                    .find(|col| **col != CompareCol::RankBar)
                    .copied()
            });
        match drop {
            Some(col) => chosen.retain(|c| *c != col),
            None => break,
        }
    }
    if !chosen.contains(&CompareCol::RankBar) {
        chosen.push(CompareCol::RankBar);
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_adds_extra_benches_on_wide_terminals() {
        let narrow = auto_columns(90);
        assert!(narrow.contains(&CompareCol::Swe));
        assert!(!narrow.contains(&CompareCol::Terminal));
        assert!(!narrow.contains(&CompareCol::LiveCode));

        let wide = auto_columns(140);
        assert!(wide.contains(&CompareCol::Terminal));
        assert!(wide.contains(&CompareCol::LiveCode));
        assert!(wide.contains(&CompareCol::Input));
    }

    #[test]
    fn fit_keeps_rank_bar_and_drops_extras_first() {
        let fitted = fit_columns(CompareCol::ALL.to_vec(), 70);
        assert!(fitted.contains(&CompareCol::RankBar));
        assert!(fitted.len() < CompareCol::ALL.len());
    }

    #[test]
    fn colset_toggle_round_trip() {
        let mut set = ColSet::empty();
        assert!(!set.contains(CompareCol::LiveCode));
        set.toggle(CompareCol::LiveCode);
        assert!(set.contains(CompareCol::LiveCode));
        set.toggle(CompareCol::LiveCode);
        assert!(!set.contains(CompareCol::LiveCode));
    }
}
