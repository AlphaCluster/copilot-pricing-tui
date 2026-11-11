//! Pricing catalog types and the on-disk JSON they load from.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const PRICING_PAGE: &str =
    "https://docs.github.com/en/copilot/reference/copilot-billing/models-and-pricing";
pub const INDIVIDUAL_PLANS_PAGE: &str =
    "https://docs.github.com/en/copilot/concepts/billing/usage-based-billing-for-individuals";
pub const ORG_PLANS_PAGE: &str = "https://docs.github.com/en/copilot/concepts/billing/usage-based-billing-for-organizations-and-enterprises";
pub const SWE_VERIFIED_PAGE: &str = "https://www.vals.ai/benchmarks/swebench";
pub const TERMINAL_BENCH_PAGE: &str = "https://www.vals.ai/benchmarks/terminal-bench-2-1";
pub const LIVECODEBENCH_PAGE: &str = "https://www.vals.ai/benchmarks/lcb";

const BUNDLED_JSON: &str = include_str!("../data/pricing.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provider {
    OpenAI,
    Anthropic,
    Google,
    GitHub,
    Microsoft,
    Xai,
    Moonshot,
    Other,
}

impl Provider {
    pub fn label(self) -> &'static str {
        match self {
            Provider::OpenAI => "OpenAI",
            Provider::Anthropic => "Anthropic",
            Provider::Google => "Google",
            Provider::GitHub => "GitHub",
            Provider::Microsoft => "Microsoft",
            Provider::Xai => "xAI",
            Provider::Moonshot => "Moonshot",
            Provider::Other => "Other",
        }
    }

    pub fn parse(raw: &str) -> Self {
        let key: String = raw
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect();
        match key.as_str() {
            "openai" => Provider::OpenAI,
            "anthropic" => Provider::Anthropic,
            "google" => Provider::Google,
            "github" | "finetunedgithub" | "finetuned" => Provider::GitHub,
            "microsoft" => Provider::Microsoft,
            "xai" => Provider::Xai,
            "moonshot" | "moonshotai" => Provider::Moonshot,
            _ => Provider::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    Lightweight,
    Versatile,
    Powerful,
}

impl Category {
    pub const ALL: [Category; 3] = [
        Category::Lightweight,
        Category::Versatile,
        Category::Powerful,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Lightweight => "Lightweight",
            Category::Versatile => "Versatile",
            Category::Powerful => "Powerful",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Category::Lightweight => "light",
            Category::Versatile => "versatile",
            Category::Powerful => "powerful",
        }
    }

    pub fn parse(raw: &str) -> Self {
        let key = raw.to_ascii_lowercase();
        if key.contains("light") {
            Category::Lightweight
        } else if key.contains("power") {
            Category::Powerful
        } else {
            Category::Versatile
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseStatus {
    Ga,
    PublicPreview,
}

impl ReleaseStatus {
    pub fn label(self) -> &'static str {
        match self {
            ReleaseStatus::Ga => "GA",
            ReleaseStatus::PublicPreview => "preview",
        }
    }

    pub fn parse(raw: &str) -> Self {
        let key = raw.to_ascii_lowercase();
        if key.contains("preview") {
            ReleaseStatus::PublicPreview
        } else {
            ReleaseStatus::Ga
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    Standard,
    LongContext,
}

impl Tier {
    pub fn parse(raw: &str) -> Self {
        if raw.to_ascii_lowercase().contains("long") {
            Tier::LongContext
        } else {
            Tier::Standard
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub variant: String,
    pub provider: Provider,
    pub category: Category,
    pub status: ReleaseStatus,
    pub tier: Tier,
    pub threshold: Option<u32>,
    pub input: f64,
    pub cached: f64,
    pub cache_write: Option<f64>,
    pub output: f64,
    pub notes: Option<String>,
    /// Short promo label when the listed rates are temporary, e.g. "through Dec 31, 2026".
    #[serde(default)]
    pub promo: Option<String>,
}

impl Model {
    pub fn is_promo(&self) -> bool {
        self.promo_label().is_some()
    }

    pub fn promo_label(&self) -> Option<&str> {
        if let Some(promo) = self.promo.as_deref().filter(|s| !s.is_empty()) {
            return Some(promo);
        }
        let note = self.notes.as_deref()?;
        let lower = note.to_ascii_lowercase();
        if !lower.contains("promo") {
            return None;
        }
        if let Some(idx) = lower.find("through ") {
            let rest = note[idx..].trim_end_matches('.').trim();
            if !rest.is_empty() {
                return Some(rest);
            }
        }
        Some("promotional pricing")
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Benchmarks {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub fetched_at: Option<String>,
    #[serde(default)]
    pub scores: Vec<BenchScore>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchScore {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub swe_verified: Option<f64>,
    #[serde(default)]
    pub terminal_bench: Option<f64>,
    #[serde(default)]
    pub livecodebench: Option<f64>,
}

impl BenchScore {
    pub fn primary(&self) -> Option<f64> {
        self.swe_verified
            .or(self.terminal_bench)
            .or(self.livecodebench)
    }
}

impl Model {
    pub fn label(&self) -> String {
        if self.variant.is_empty() {
            self.name.clone()
        } else {
            format!("{}  ·  {}", self.name, self.variant)
        }
    }

    pub fn short_label(&self) -> String {
        if self.variant.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({})", self.name, self.variant)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub name: String,
    pub price_usd: f64,
    pub credits: u32,
    pub detail: String,
}

impl Plan {
    pub fn usd_allowance(&self) -> f64 {
        f64::from(self.credits) / 100.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Scenario {
    pub name: &'static str,
    pub blurb: &'static str,
    pub input: u64,
    pub cached: u64,
    pub cache_write: u64,
    pub output: u64,
}

impl Scenario {
    pub fn usage(self) -> Usage {
        Usage {
            input: self.input,
            cached: self.cached,
            cache_write: self.cache_write,
            output: self.output,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Usage {
    pub input: u64,
    pub cached: u64,
    pub cache_write: u64,
    pub output: u64,
}

impl Usage {
    pub fn total_tokens(self) -> u64 {
        self.input
            .saturating_add(self.cached)
            .saturating_add(self.cache_write)
            .saturating_add(self.output)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Catalog {
    pub version: u32,
    pub source: String,
    pub fetched_at: Option<String>,
    pub models: Vec<Model>,
    pub plans: Vec<Plan>,
    #[serde(default)]
    pub benchmarks: Benchmarks,
    #[serde(skip)]
    pub path: Option<PathBuf>,
}

impl Catalog {
    pub fn bundled() -> Self {
        Self::from_json(BUNDLED_JSON).expect("bundled pricing.json is valid")
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let catalog: Catalog =
            serde_json::from_str(text).map_err(|err| format!("invalid pricing.json: {err}"))?;
        if catalog.models.is_empty() {
            return Err("pricing.json contains no models".into());
        }
        Ok(catalog)
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|err| err.to_string())
    }

    pub fn load() -> Self {
        for path in candidate_read_paths() {
            if let Ok(text) = fs::read_to_string(&path)
                && let Ok(mut catalog) = Self::from_json(&text)
            {
                catalog.path = Some(path);
                return catalog;
            }
        }
        Self::bundled()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create {}: {err}", parent.display()))?;
        }
        fs::write(path, self.to_json()?).map_err(|err| format!("write {}: {err}", path.display()))
    }

    pub fn model_by_id(&self, id: &str) -> Option<&Model> {
        self.models.iter().find(|model| model.id == id)
    }

    pub fn providers(&self) -> Vec<Provider> {
        let mut out = Vec::new();
        for model in &self.models {
            if !out.contains(&model.provider) {
                out.push(model.provider);
            }
        }
        out
    }

    pub fn bench_for(&self, model: &Model) -> Option<&BenchScore> {
        let name_key = family_key(&model.name);
        let id_key = family_key(&model.id);
        self.benchmarks
            .scores
            .iter()
            .filter(|score| {
                let key = family_key(&score.slug);
                key == name_key || key == id_key
            })
            .min_by_key(|score| {
                let missing = [
                    score.swe_verified,
                    score.terminal_bench,
                    score.livecodebench,
                ]
                .iter()
                .filter(|value| value.is_none())
                .count();
                (missing, extra_suffix_penalty(&score.slug))
            })
    }

    pub fn scored_count(&self) -> usize {
        self.models
            .iter()
            .filter(|model| self.bench_for(model).is_some())
            .count()
    }
}

/// Collapse a model name or Vals slug into a comparable family key.
pub fn family_key(raw: &str) -> String {
    let tail = raw.rsplit('/').next().unwrap_or(raw);
    let stripped = strip_embedded_dates(tail);
    let mut key: String = stripped
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect();
    for suffix in ["claudecode", "thinking", "preview", "factory"] {
        if let Some(stripped) = key.strip_suffix(suffix) {
            key = stripped.to_string();
        }
    }
    key
}

fn strip_embedded_dates(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if is_iso_date(&chars[i..]) {
            i += 10;
            continue;
        }
        if is_yyyymmdd(&chars[i..]) {
            i += 8;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn is_iso_date(chars: &[char]) -> bool {
    chars.len() >= 10
        && chars[0] == '2'
        && chars[1] == '0'
        && chars[2].is_ascii_digit()
        && chars[3].is_ascii_digit()
        && chars[4] == '-'
        && chars[5].is_ascii_digit()
        && chars[6].is_ascii_digit()
        && chars[7] == '-'
        && chars[8].is_ascii_digit()
        && chars[9].is_ascii_digit()
}

fn is_yyyymmdd(chars: &[char]) -> bool {
    chars.len() >= 8
        && chars[0] == '2'
        && chars[1] == '0'
        && chars[2..8].iter().all(|ch| ch.is_ascii_digit())
}

fn extra_suffix_penalty(slug: &str) -> i32 {
    let tail = slug.rsplit('/').next().unwrap_or(slug).to_ascii_lowercase();
    ["claude-code", "thinking", "preview", "factory", "codex-max"]
        .iter()
        .filter(|suffix| tail.contains(*suffix))
        .count() as i32
}

/// Representative token mixes used by the estimator. These are ours, not GitHub's.
pub const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "Quick chat",
        blurb: "A short ask/edit turn with little context.",
        input: 2_000,
        cached: 400,
        cache_write: 0,
        output: 400,
    },
    Scenario {
        name: "Coding chat",
        blurb: "A typical IDE chat turn with open files in context.",
        input: 12_000,
        cached: 6_000,
        cache_write: 1_500,
        output: 2_000,
    },
    Scenario {
        name: "Agent session",
        blurb: "A multi-step agent run across several files.",
        input: 80_000,
        cached: 40_000,
        cache_write: 15_000,
        output: 8_000,
    },
    Scenario {
        name: "Long context",
        blurb: "A large-repo turn that crosses long-context thresholds.",
        input: 350_000,
        cached: 80_000,
        cache_write: 20_000,
        output: 12_000,
    },
];

/// Default catalog "typical cost" mix — same as the coding-chat scenario.
pub const TYPICAL_USAGE: Usage = Usage {
    input: 12_000,
    cached: 6_000,
    cache_write: 1_500,
    output: 2_000,
};

pub fn default_plans() -> Vec<Plan> {
    vec![
        Plan {
            name: "Pro".into(),
            price_usd: 10.0,
            credits: 1_500,
            detail: "$10 base + $5 flex".into(),
        },
        Plan {
            name: "Pro+".into(),
            price_usd: 39.0,
            credits: 7_000,
            detail: "$39 base + $31 flex".into(),
        },
        Plan {
            name: "Max".into(),
            price_usd: 100.0,
            credits: 20_000,
            detail: "$100 base + $100 flex".into(),
        },
        Plan {
            name: "Business".into(),
            price_usd: 19.0,
            credits: 1_900,
            detail: "1,900 credits / user, pooled".into(),
        },
        Plan {
            name: "Enterprise".into(),
            price_usd: 39.0,
            credits: 3_900,
            detail: "3,900 credits / user, pooled".into(),
        },
    ]
}

pub fn preferred_write_path() -> PathBuf {
    if let Ok(path) = env::var("COPILOT_PRICING_DATA") {
        return PathBuf::from(path);
    }
    let local = PathBuf::from("data/pricing.json");
    if local.exists() || Path::new("data").is_dir() || Path::new("Cargo.toml").exists() {
        return local;
    }
    user_data_file().unwrap_or(local)
}

fn candidate_read_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(path) = env::var("COPILOT_PRICING_DATA") {
        paths.push(PathBuf::from(path));
    }
    paths.push(PathBuf::from("data/pricing.json"));
    if let Ok(exe) = env::current_exe()
        && let Some(dir) = exe.parent()
    {
        paths.push(dir.join("data/pricing.json"));
        paths.push(dir.join("pricing.json"));
    }
    if let Some(user) = user_data_file() {
        paths.push(user);
    }
    paths
}

fn user_data_file() -> Option<PathBuf> {
    if let Ok(dir) = env::var("XDG_DATA_HOME") {
        return Some(PathBuf::from(dir).join("copilot-pricing/pricing.json"));
    }
    let home = env::var_os("HOME")?;
    let base = PathBuf::from(home);
    if cfg!(target_os = "macos") {
        Some(base.join("Library/Application Support/copilot-pricing/pricing.json"))
    } else {
        Some(base.join(".local/share/copilot-pricing/pricing.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn bundled_catalog_is_usable() {
        let catalog = Catalog::bundled();
        let mut seen = HashSet::new();
        assert!(catalog.models.len() >= 10);
        for model in &catalog.models {
            assert!(seen.insert(model.id.as_str()), "duplicate id {}", model.id);
            assert!(model.input > 0.0, "{}", model.id);
            assert!(model.cached > 0.0, "{}", model.id);
            assert!(model.output > 0.0, "{}", model.id);
        }
        assert!(!catalog.plans.is_empty());
    }

    #[test]
    fn provider_parse() {
        assert_eq!(Provider::parse("OpenAI"), Provider::OpenAI);
        assert_eq!(Provider::parse("fine-tuned-github"), Provider::GitHub);
        assert_eq!(Provider::parse("Moonshot AI"), Provider::Moonshot);
        assert_eq!(Provider::parse("mystery"), Provider::Other);
    }

    #[test]
    fn family_keys_align_vals_slugs_with_copilot_names() {
        let pairs = [
            ("GPT-5.6 Sol", "openai/gpt-5.6-sol"),
            ("GPT-5.4 mini", "openai/gpt-5.4-mini-2026-03-17"),
            ("GPT-5 mini", "openai/gpt-5-mini-2025-08-07"),
            ("Claude Opus 4.8", "anthropic/claude-opus-4-8"),
            (
                "Claude Opus 4.5",
                "anthropic/claude-opus-4-5-20251101-thinking",
            ),
            ("Gemini 3.1 Pro", "google/gemini-3.1-pro-preview"),
            ("Kimi K2.7 Code", "kimi/kimi-k2.7-code"),
            ("Grok 4.6", "grok/grok-4.6"),
            ("GPT-5.3-Codex", "openai/gpt-5.3-codex"),
        ];
        for (name, slug) in pairs {
            assert_eq!(family_key(name), family_key(slug), "{name} vs {slug}");
        }
        assert_ne!(family_key("GPT-5.4"), family_key("GPT-5.4 mini"));
    }

    #[test]
    fn bench_for_prefers_complete_canonical_slug() {
        let catalog = Catalog {
            version: 1,
            source: String::new(),
            fetched_at: None,
            models: vec![],
            plans: vec![],
            benchmarks: Benchmarks {
                source: String::new(),
                fetched_at: None,
                scores: vec![
                    BenchScore {
                        slug: "anthropic/claude-opus-4-8-claude-code".into(),
                        name: "Opus 4.8 code".into(),
                        swe_verified: Some(85.8),
                        terminal_bench: Some(69.0),
                        livecodebench: None,
                    },
                    BenchScore {
                        slug: "anthropic/claude-opus-4-8".into(),
                        name: "Opus 4.8".into(),
                        swe_verified: Some(88.6),
                        terminal_bench: Some(71.9),
                        livecodebench: Some(87.8),
                    },
                ],
            },
            path: None,
        };
        let model = Model {
            id: "claude-opus-4.8".into(),
            name: "Claude Opus 4.8".into(),
            variant: String::new(),
            provider: Provider::Anthropic,
            category: Category::Powerful,
            status: ReleaseStatus::Ga,
            tier: Tier::Standard,
            threshold: None,
            input: 5.0,
            cached: 0.5,
            cache_write: None,
            output: 25.0,
            notes: None,
            promo: None,
        };
        let score = catalog.bench_for(&model).unwrap();
        assert_eq!(score.swe_verified, Some(88.6));
    }

    #[test]
    fn promo_label_reads_through_date_from_notes() {
        let mut model = Model {
            id: "gemini-3.6-flash".into(),
            name: "Gemini 3.6 Flash".into(),
            variant: String::new(),
            provider: Provider::Google,
            category: Category::Versatile,
            status: ReleaseStatus::Ga,
            tier: Tier::Standard,
            threshold: None,
            input: 0.75,
            cached: 0.075,
            cache_write: None,
            output: 3.75,
            notes: Some(
                "Promotional pricing of $0.75 per 1M input tokens through December 31, 2026."
                    .into(),
            ),
            promo: None,
        };
        assert_eq!(model.promo_label(), Some("through December 31, 2026"));
        assert!(model.is_promo());
        model.notes = None;
        assert!(!model.is_promo());
    }
}
