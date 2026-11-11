# copilot-pricing

A terminal UI for comparing [GitHub Copilot](https://docs.github.com/en/copilot/reference/copilot-billing/models-and-pricing) model prices.

The docs page is a long set of per-provider tables. This app puts every billed SKU in one catalog, lets you select several at once, and then shows ranked bars plus a token-mix estimator against your monthly AI-credit allowance.

Prices are **USD per 1 million tokens**. **1 AI credit = $0.01**. Code completions stay unlimited on paid plans and are not billed here.

## Run

```bash
cargo run --release
```

Requires a reasonably wide terminal (about 100 columns is comfortable).

Model rates come from `data/pricing.json`. Refresh them from the official docs page when GitHub changes prices:

```bash
cargo run --release -- update
```

Or press `u` in the TUI. The updater fetches the published tables at [Models and pricing](https://docs.github.com/en/copilot/reference/copilot-billing/models-and-pricing) plus the individual/org plan allowance pages, then rewrites the JSON. Set `COPILOT_PRICING_DATA` to use a different file. If no file is present the app falls back to the JSON bundled at compile time.

Selected models persist in `data/session.json` (or next to your pricing file). Press `x` to clear the comparison.

## Views

1. **Catalog** — every model row from the official pricing tables, including default vs long-context tiers, plus SWE-bench Verified / Terminal-Bench 2.1 / LiveCodeBench where Vals has a score. Search, filter by provider or class, sort, and multi-select.
2. **Compare** — selected models ranked on a metric you choose. Column layout follows window width; `v` cycles presets (auto / pricing / benches / full), `o` toggles individual columns, and `b` shows or hides Terminal-Bench and LiveCodeBench.
3. **Estimate** — pick a token mix (quick chat, coding chat, agent session, long context) or edit the counts. See USD, credits, percent of plan, and how many of those turns fit in the month.

If nothing is selected, Compare and Estimate use the currently visible catalog set.

## Keys

| Key | Action |
| --- | --- |
| `1` `2` `3` / `Tab` | Switch views |
| `j` `k` / arrows | Move |
| `Space` | Select or deselect a model |
| `Enter` | Open Compare |
| `a` / `x` | Select visible / clear comparison (saved between sessions) |
| `/` | Search |
| `f` `c` `s` | Provider, category, sort (`S` reverses) |
| `m` | Compare metric |
| `v` | Cycle compare column layout |
| `o` | Hide or show compare columns |
| `b` | Toggle extra benchmark columns |
| `n` | Token preset |
| `,` `.` and `h` `l` | Token field and value (`H` `L` coarse) |
| `p` | Cycle plan (Pro, Pro+, Max, Business, Enterprise) |
| `d` | Toggle the 10% auto-model-selection discount |
| `t` / `T` | Cycle theme |
| `u` | Refresh rates from GitHub docs |
| `?` | Help |
| `q` | Quit |

## Plans (checked 2026-11-10)

Prices and allowances below match [Plans for GitHub Copilot](https://docs.github.com/en/copilot/get-started/plans). `p` cycles only the plans that include a monthly allowance. Copilot Free and Copilot Student have an unpublished credit allowance and are not listed.

| Plan | Price | Monthly AI credits |
| --- | ---: | ---: |
| Pro | $10 | 1,500 ($10 base + $5 flex) |
| Pro+ | $39 | 7,000 ($39 base + $31 flex) |
| Max | $100 | 20,000 ($100 base + $100 flex) |
| Business | $19 / user | 1,900, pooled |
| Enterprise | $39 / user | 3,900, pooled |
| Copilot access | $0 with a GitHub Enterprise Cloud license, or $10 / user as Copilot Standalone | None. Usage is billed from the first request |

Base credits match the subscription price and do not change. Flex credits are extra and GitHub can change them. Business and Enterprise credits are pooled across the billing entity. Copilot access is for eligible enterprises only and is left out of the estimator: with no included credits, percent-of-plan and turns-per-month would both be zero.

Paid plans get 10% off model cost when auto model selection is on in Copilot Chat or the Copilot cloud agent. Press `d` to apply that discount. Through November 30, 2026, Copilot Fusion is a separate 30% discount on model costs. The estimator does not apply the Fusion rate.

## Data

The TUI does not hardcode prices. It reads `data/pricing.json`. Model rates come from GitHub's published tables; SWE scores come from [Vals AI](https://www.vals.ai/benchmarks/swebench) using mini-SWE-agent so models share the same harness:

- [Models and pricing](https://docs.github.com/en/copilot/reference/copilot-billing/models-and-pricing)
- [Plans for GitHub Copilot](https://docs.github.com/en/copilot/get-started/plans)
- [Usage-based billing for individuals](https://docs.github.com/en/copilot/concepts/billing/usage-based-billing-for-individuals)
- [Usage-based billing for organizations](https://docs.github.com/en/copilot/concepts/billing/usage-based-billing-for-organizations-and-enterprises)
- [SWE-bench Verified](https://www.vals.ai/benchmarks/swebench), [Terminal-Bench 2.1](https://www.vals.ai/benchmarks/terminal-bench-2-1), [LiveCodeBench](https://www.vals.ai/benchmarks/lcb)

`cargo test` covers the HTML table parser and cost math. After `update`, glance at the JSON if a new table layout lands on the docs page.
