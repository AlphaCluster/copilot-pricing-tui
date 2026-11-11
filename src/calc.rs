//! Cost math for Copilot token pricing.

use crate::data::{Model, Plan, Usage};

/// Paid Copilot plans get 10% off when auto model selection is used
/// in Copilot Chat, Copilot CLI, the Copilot app, or cloud agent.
pub const AUTO_DISCOUNT: f64 = 0.10;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cost {
    pub usd: f64,
    pub credits: f64,
}

impl Cost {
    pub fn from_usd(usd: f64) -> Self {
        Self {
            usd,
            credits: usd * 100.0,
        }
    }

    pub fn times(self, n: f64) -> Self {
        Self::from_usd(self.usd * n)
    }
}

pub fn interaction_cost(model: &Model, usage: Usage, auto_discount: bool) -> Cost {
    let write_rate = model.cache_write.unwrap_or(0.0);
    let mut usd = (usage.input as f64 / 1_000_000.0) * model.input
        + (usage.cached as f64 / 1_000_000.0) * model.cached
        + (usage.cache_write as f64 / 1_000_000.0) * write_rate
        + (usage.output as f64 / 1_000_000.0) * model.output;
    if auto_discount {
        usd *= 1.0 - AUTO_DISCOUNT;
    }
    Cost::from_usd(usd)
}

/// How many of this interaction fit in the plan's monthly credits.
pub fn interactions_per_plan(cost: Cost, plan: &Plan) -> f64 {
    if cost.credits <= 0.0 {
        f64::INFINITY
    } else {
        f64::from(plan.credits) / cost.credits
    }
}

/// Share of the monthly allowance burned by one interaction, 0–100+.
pub fn plan_percent(cost: Cost, plan: &Plan) -> f64 {
    if plan.credits == 0 {
        0.0
    } else {
        cost.credits / f64::from(plan.credits) * 100.0
    }
}

/// Relative multiplier vs a baseline cost (1.0 = same price).
pub fn vs_baseline(cost: Cost, baseline: Cost) -> f64 {
    if baseline.usd <= 0.0 {
        1.0
    } else {
        cost.usd / baseline.usd
    }
}

pub fn metric_value(
    model: &Model,
    usage: Usage,
    auto_discount: bool,
    metric: CompareMetric,
    bench: Option<&crate::data::BenchScore>,
) -> f64 {
    match metric {
        CompareMetric::Typical => interaction_cost(model, usage, auto_discount).usd,
        CompareMetric::Input => model.input,
        CompareMetric::Cached => model.cached,
        CompareMetric::CacheWrite => model.cache_write.unwrap_or(0.0),
        CompareMetric::Output => model.output,
        CompareMetric::SweVerified => bench.and_then(|b| b.swe_verified).unwrap_or(f64::NAN),
        CompareMetric::TerminalBench => bench.and_then(|b| b.terminal_bench).unwrap_or(f64::NAN),
        CompareMetric::LiveCode => bench.and_then(|b| b.livecodebench).unwrap_or(f64::NAN),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareMetric {
    Typical,
    Input,
    Cached,
    CacheWrite,
    Output,
    SweVerified,
    TerminalBench,
    LiveCode,
}

impl CompareMetric {
    pub const ALL: [CompareMetric; 8] = [
        CompareMetric::Typical,
        CompareMetric::Input,
        CompareMetric::Cached,
        CompareMetric::CacheWrite,
        CompareMetric::Output,
        CompareMetric::SweVerified,
        CompareMetric::TerminalBench,
        CompareMetric::LiveCode,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CompareMetric::Typical => "typical turn",
            CompareMetric::Input => "input / 1M",
            CompareMetric::Cached => "cached / 1M",
            CompareMetric::CacheWrite => "cache write / 1M",
            CompareMetric::Output => "output / 1M",
            CompareMetric::SweVerified => "SWE-bench Verified",
            CompareMetric::TerminalBench => "Terminal-Bench 2.1",
            CompareMetric::LiveCode => "LiveCodeBench",
        }
    }

    pub fn higher_better(self) -> bool {
        matches!(
            self,
            CompareMetric::SweVerified | CompareMetric::TerminalBench | CompareMetric::LiveCode
        )
    }

    /// Total order for ranking. Missing scores sort last; never returns NaN.
    pub fn rank_key(self, value: f64) -> (u8, i64) {
        if value.is_finite() {
            let scaled = (value * 1_000_000.0).round() as i64;
            (
                0,
                if self.higher_better() {
                    -scaled
                } else {
                    scaled
                },
            )
        } else {
            (1, 0)
        }
    }

    pub fn next(self) -> Self {
        cycle_next(self, &Self::ALL)
    }

    pub fn prev(self) -> Self {
        cycle_prev(self, &Self::ALL)
    }
}

fn cycle_next<T: Copy + PartialEq>(current: T, items: &[T]) -> T {
    match items.iter().position(|item| *item == current) {
        Some(i) => items[(i + 1) % items.len()],
        None => items[0],
    }
}

fn cycle_prev<T: Copy + PartialEq>(current: T, items: &[T]) -> T {
    match items.iter().position(|item| *item == current) {
        Some(i) => items[(i + items.len() - 1) % items.len()],
        None => items[0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Catalog, Plan, TYPICAL_USAGE};

    fn almost(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    fn catalog() -> Catalog {
        Catalog::bundled()
    }

    #[test]
    fn million_tokens_equals_list_price() {
        let catalog = catalog();
        let model = catalog.model_by_id("gpt-5.4-nano").unwrap();
        let cost = interaction_cost(
            model,
            Usage {
                input: 1_000_000,
                cached: 1_000_000,
                cache_write: 1_000_000,
                output: 1_000_000,
            },
            false,
        );
        // no cache-write rate, so 0.20 + 0.02 + 1.25
        almost(cost.usd, 1.47);
        almost(cost.credits, 147.0);
    }

    #[test]
    fn cache_write_is_billed_when_present() {
        let catalog = catalog();
        let model = catalog.model_by_id("claude-sonnet-5").unwrap();
        let cost = interaction_cost(
            model,
            Usage {
                input: 1_000_000,
                cached: 0,
                cache_write: 1_000_000,
                output: 0,
            },
            false,
        );
        almost(cost.usd, 2.00 + 2.50);
    }

    #[test]
    fn auto_discount_is_ten_percent() {
        let catalog = catalog();
        let model = catalog.model_by_id("gpt-5.4-nano").unwrap();
        let full = interaction_cost(model, TYPICAL_USAGE, false);
        let discounted = interaction_cost(model, TYPICAL_USAGE, true);
        almost(discounted.usd, full.usd * 0.9);
    }

    #[test]
    fn typical_coding_chat_gpt54_nano() {
        let catalog = catalog();
        let model = catalog.model_by_id("gpt-5.4-nano").unwrap();
        let cost = interaction_cost(model, TYPICAL_USAGE, false);
        // 12k*0.20 + 6k*0.02 + 1.5k*0 + 2k*1.25 per million
        almost(cost.usd, 0.0024 + 0.00012 + 0.0025);
        almost(cost.credits, 0.502);
    }

    #[test]
    fn plan_yield_and_percent() {
        let plan = Plan {
            name: "Pro".into(),
            price_usd: 10.0,
            credits: 1_500,
            detail: String::new(),
        };
        let cost = Cost::from_usd(0.15); // 15 credits
        almost(interactions_per_plan(cost, &plan), 100.0);
        almost(plan_percent(cost, &plan), 1.0);
    }

    #[test]
    fn credits_are_cents() {
        almost(Cost::from_usd(1.23).credits, 123.0);
    }

    #[test]
    fn ranking_missing_swe_scores_is_a_total_order() {
        let values = [f64::NAN, 96.2, f64::NAN, 70.0, 88.6, f64::NAN];
        let mut keys: Vec<_> = values
            .iter()
            .map(|v| CompareMetric::SweVerified.rank_key(*v))
            .collect();
        keys.sort();
        assert!(keys.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(keys[0], CompareMetric::SweVerified.rank_key(96.2));
        assert!(keys[3..].iter().all(|k| k.0 == 1));
    }
}
