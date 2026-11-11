//! Pull Copilot pricing tables from GitHub Docs and write `data/pricing.json`.

use crate::data::{
    BenchScore, Benchmarks, Catalog, Category, INDIVIDUAL_PLANS_PAGE, LIVECODEBENCH_PAGE, Model,
    ORG_PLANS_PAGE, PRICING_PAGE, Plan, Provider, ReleaseStatus, SWE_VERIFIED_PAGE,
    TERMINAL_BENCH_PAGE, Tier, default_plans,
};

const USER_AGENT: &str = "copilot-pricing/0.1 (+https://docs.github.com/en/copilot/reference/copilot-billing/models-and-pricing)";

pub fn update_from_docs() -> Result<Catalog, String> {
    let pricing_html = fetch(PRICING_PAGE)?;
    let mut models = parse_pricing_tables(&pricing_html)?;
    if models.is_empty() {
        return Err("docs page contained no model rows".into());
    }

    let mut plans = default_plans();
    if let Ok(individual) = fetch(INDIVIDUAL_PLANS_PAGE) {
        merge_plans(&mut plans, parse_individual_plans(&individual));
    }
    if let Ok(orgs) = fetch(ORG_PLANS_PAGE) {
        merge_plans(&mut plans, parse_org_plans(&orgs));
    }

    dedupe_model_ids(&mut models);

    let previous = Catalog::load();
    let benchmarks = match fetch_benchmarks() {
        Ok(benches) if !benches.scores.is_empty() => benches,
        Ok(_) => previous.benchmarks,
        Err(_) => previous.benchmarks,
    };

    Ok(Catalog {
        version: 1,
        source: PRICING_PAGE.to_string(),
        fetched_at: Some(now_stamp()),
        models,
        plans,
        benchmarks,
        path: None,
    })
}

pub fn fetch_benchmarks() -> Result<Benchmarks, String> {
    let mut scores: Vec<BenchScore> = Vec::new();
    let mut sources = Vec::new();

    if let Ok(html) = fetch(SWE_VERIFIED_PAGE) {
        apply_vals_scores(&mut scores, &html, BenchKind::SweVerified);
        sources.push("SWE-bench Verified");
    }
    if let Ok(html) = fetch(TERMINAL_BENCH_PAGE) {
        apply_vals_scores(&mut scores, &html, BenchKind::TerminalBench);
        sources.push("Terminal-Bench 2.1");
    }
    if let Ok(html) = fetch(LIVECODEBENCH_PAGE) {
        apply_vals_scores(&mut scores, &html, BenchKind::LiveCode);
        sources.push("LiveCodeBench");
    }
    if scores.is_empty() {
        return Err("no SWE benchmark scores parsed from Vals AI".into());
    }
    Ok(Benchmarks {
        source: format!("https://www.vals.ai ({})", sources.join(", ")),
        fetched_at: Some(now_stamp()),
        scores,
    })
}

#[derive(Clone, Copy)]
enum BenchKind {
    SweVerified,
    TerminalBench,
    LiveCode,
}

fn apply_vals_scores(scores: &mut Vec<BenchScore>, html: &str, kind: BenchKind) {
    for (slug, accuracy) in parse_vals_accuracies(html) {
        if let Some(existing) = scores.iter_mut().find(|score| score.slug == slug) {
            match kind {
                BenchKind::SweVerified => existing.swe_verified = Some(accuracy),
                BenchKind::TerminalBench => existing.terminal_bench = Some(accuracy),
                BenchKind::LiveCode => existing.livecodebench = Some(accuracy),
            }
        } else {
            let mut score = BenchScore {
                name: pretty_vals_slug(&slug),
                slug,
                swe_verified: None,
                terminal_bench: None,
                livecodebench: None,
            };
            match kind {
                BenchKind::SweVerified => score.swe_verified = Some(accuracy),
                BenchKind::TerminalBench => score.terminal_bench = Some(accuracy),
                BenchKind::LiveCode => score.livecodebench = Some(accuracy),
            }
            scores.push(score);
        }
    }
}

/// First overall accuracy per Vals model slug.
pub fn parse_vals_accuracies(html: &str) -> Vec<(String, f64)> {
    let text = decode_entities(html);
    let needle = ":[0,{\"accuracy\":[0,";
    let mut first = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut from = 0;
    while let Some(rel) = text[from..].find(needle) {
        let at = from + rel;
        let before = &text[..at];
        if let Some(end_quote) = before.rfind('"')
            && let Some(start_quote) = before[..end_quote].rfind('"')
        {
            let slug = &before[start_quote + 1..end_quote];
            if slug.contains('/') && !slug.contains(' ') && seen.insert(slug.to_string()) {
                let num_at = at + needle.len();
                let num_len = text[num_at..]
                    .find(|ch: char| !ch.is_ascii_digit() && ch != '.')
                    .unwrap_or(0);
                if let Ok(accuracy) = text[num_at..num_at + num_len].parse::<f64>() {
                    first.push((slug.to_string(), accuracy));
                }
            }
        }
        from = at + needle.len();
    }
    first
}

fn pretty_vals_slug(slug: &str) -> String {
    let tail = slug.rsplit('/').next().unwrap_or(slug);
    let mut parts = Vec::new();
    for part in tail.split('-') {
        if part.is_empty() {
            continue;
        }
        if part.chars().all(|ch| ch.is_ascii_digit()) && part.len() >= 6 {
            continue;
        }
        if matches!(part, "thinking" | "preview") {
            continue;
        }
        if part.chars().next().is_some_and(|ch| ch.is_ascii_digit()) || part.contains('.') {
            parts.push(part.to_string());
        } else if part.eq_ignore_ascii_case("gpt") {
            parts.push("GPT".into());
        } else {
            let mut chars = part.chars();
            let first = chars.next().map(|ch| ch.to_ascii_uppercase());
            parts.push(format!("{}{}", first.unwrap_or_default(), chars.as_str()));
        }
    }
    parts.join(" ")
}

pub fn refresh_and_save() -> Result<(Catalog, std::path::PathBuf), String> {
    let mut catalog = update_from_docs()?;
    let path = crate::data::preferred_write_path();
    catalog.save(&path)?;
    catalog.path = Some(path.clone());
    Ok((catalog, path))
}

fn fetch(url: &str) -> Result<String, String> {
    // ureq 2 timed out each phase after 30s. ureq 3 waits forever unless a timeout is set.
    let mut response = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "text/html")
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .call()
        .map_err(|err| format!("fetch {url}: {err}"))?;
    response
        .body_mut()
        .read_to_string()
        .map_err(|err| format!("read {url}: {err}"))
}

pub fn parse_pricing_tables(html: &str) -> Result<Vec<Model>, String> {
    let footnotes = parse_footnotes(html);
    let mut models = Vec::new();
    for table in extract_tables(html) {
        let Some(provider) = provider_for_table(&table) else {
            continue;
        };
        if !is_model_table(&table.headers) {
            continue;
        }
        for row in &table.rows {
            if let Some(model) = row_to_model(provider, &table.headers, row, &footnotes) {
                models.push(model);
            }
        }
    }
    Ok(models)
}

fn provider_for_table(table: &HtmlTable) -> Option<Provider> {
    let label = table
        .aria
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(table.caption.as_str());
    let parsed = Provider::parse(label);
    if parsed == Provider::Other && label.is_empty() {
        None
    } else {
        Some(parsed)
    }
}

fn header_matches(header: &str, needle: &str) -> bool {
    let header = header.to_ascii_lowercase();
    match needle {
        "model" => header == "model" || header.starts_with("model "),
        "input" => header == "input" || header == "input tokens",
        "cached" => header.contains("cached"),
        "cache write" => header.contains("write"),
        "output" => header.contains("output"),
        "release" => header.contains("release") || header.contains("status"),
        "category" => header.contains("category"),
        "tier" => header == "tier" || header.contains("tier"),
        "threshold" => header.contains("threshold"),
        _ => header.contains(needle),
    }
}

fn is_model_table(headers: &[String]) -> bool {
    let joined = headers.join(" ").to_ascii_lowercase();
    joined.contains("model") && joined.contains("input") && joined.contains("output")
}

fn row_to_model(
    provider: Provider,
    headers: &[String],
    row: &[String],
    footnotes: &[(String, String)],
) -> Option<Model> {
    let get = |needle: &str| -> String {
        headers
            .iter()
            .zip(row.iter())
            .find(|(header, _)| header_matches(header, needle))
            .map(|(_, value)| value.trim().to_string())
            .unwrap_or_default()
    };

    let raw_name = get("model");
    if raw_name.is_empty() {
        return None;
    }

    let note_from_fn = footnote_for(&raw_name, footnotes);
    let name = clean_model_name(&raw_name);
    if name.is_empty() {
        return None;
    }

    let input = parse_money(&get("input"))?;
    let cached = parse_money(&get("cached"))?;
    let output = parse_money(&get("output"))?;
    let cache_write = {
        let raw = get("cache write");
        if raw.is_empty() {
            None
        } else {
            parse_money(&raw)
        }
    };

    let status = ReleaseStatus::parse(&get("release"));
    let category = Category::parse(&get("category"));
    let tier = Tier::parse(&get("tier"));
    let threshold_raw = get("threshold");
    let threshold = parse_threshold(&threshold_raw);
    let variant = variant_label(&name, tier, &threshold_raw);
    let id = make_id(&name, &variant, tier);

    let mut notes = note_from_fn;
    if raw_name.to_ascii_lowercase().contains("preview") && notes.is_none() {
        notes = Some("Listed as preview on the GitHub docs page.".into());
    }
    let promo = extract_promo(notes.as_deref().unwrap_or(&raw_name));

    Some(Model {
        id,
        name: strip_fast_mode(&name),
        variant,
        provider,
        category,
        status,
        tier,
        threshold,
        input,
        cached,
        cache_write,
        output,
        notes,
        promo,
    })
}

pub fn extract_promo(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    if !lower.contains("promo") {
        return None;
    }
    if let Some(idx) = lower.find("through ") {
        let rest = text[idx..].trim();
        let end = rest.find('.').unwrap_or(rest.len().min(56));
        let label = rest[..end].trim();
        if !label.is_empty() {
            return Some(label.to_string());
        }
    }
    Some("promotional pricing".into())
}

fn clean_model_name(raw: &str) -> String {
    let mut name = raw.to_string();
    while let Some(start) = name.find("[^") {
        if let Some(end) = name[start..].find(']') {
            name.replace_range(start..start + end + 1, "");
        } else {
            break;
        }
    }
    collapse_ws(&name)
}

fn strip_fast_mode(name: &str) -> String {
    collapse_ws(
        &name
            .replace("(fast mode)", "")
            .replace("fast mode", "")
            .replace("(preview)", "")
            .replace("(Preview)", ""),
    )
}

fn variant_label(name: &str, tier: Tier, threshold_raw: &str) -> String {
    if name.to_ascii_lowercase().contains("fast mode") {
        return "fast mode".into();
    }
    let thresh = pretty_threshold(threshold_raw);
    match tier {
        Tier::LongContext => {
            if thresh.is_empty() {
                "long context".into()
            } else {
                format!("long {thresh}")
            }
        }
        Tier::Standard if !thresh.is_empty() => format!("default {thresh}"),
        Tier::Standard => String::new(),
    }
}

fn pretty_threshold(raw: &str) -> String {
    let t = collapse_ws(raw);
    if t.is_empty() || t.eq_ignore_ascii_case("not applicable") {
        String::new()
    } else {
        t.replace(' ', "")
    }
}

fn make_id(name: &str, variant: &str, tier: Tier) -> String {
    let base = slug(&strip_fast_mode(name));
    if variant == "fast mode" {
        format!("{base}-fast")
    } else if tier == Tier::LongContext {
        format!("{base}-long")
    } else if variant.starts_with("default") {
        format!("{base}-default")
    } else {
        base
    }
}

fn slug(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' {
            out.push(ch.to_ascii_lowercase());
        } else if (ch == ' ' || ch == '-' || ch == '_' || ch == '/') && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

pub fn parse_individual_plans(html: &str) -> Vec<Plan> {
    let mut plans = Vec::new();
    for table in extract_tables(html) {
        let headers = table
            .headers
            .iter()
            .map(|h| h.to_ascii_lowercase())
            .collect::<Vec<_>>();
        if !headers.iter().any(|h| h.contains("plan")) {
            continue;
        }
        if !headers
            .iter()
            .any(|h| h.contains("credit") || h.contains("price"))
        {
            continue;
        }
        for row in &table.rows {
            if let Some(plan) = plan_from_row(&table.headers, row) {
                plans.push(plan);
            }
        }
    }
    plans
}

pub fn parse_org_plans(html: &str) -> Vec<Plan> {
    let lower = html.to_ascii_lowercase();
    let cut = [
        "promotional amounts",
        "promotional period",
        "promotional pricing",
    ]
    .into_iter()
    .filter_map(|marker| lower.find(marker))
    .min();
    let html = cut.map(|idx| &html[..idx]).unwrap_or(html);
    parse_individual_plans(html)
}

fn plan_from_row(headers: &[String], row: &[String]) -> Option<Plan> {
    let get = |needles: &[&str]| -> String {
        for needle in needles {
            if let Some((_, value)) = headers
                .iter()
                .zip(row.iter())
                .find(|(header, _)| header.to_ascii_lowercase().contains(needle))
            {
                return collapse_ws(value);
            }
        }
        String::new()
    };

    let raw_name = get(&["plan"]);
    let name = normalize_plan_name(&raw_name)?;
    let price = parse_money(&get(&["price"])).unwrap_or(0.0);
    let credits = parse_int(&get(&[
        "total monthly",
        "total ai",
        "credits per user",
        "credits",
    ]))
    .or_else(|| {
        let base = parse_int(&get(&["base"]))?;
        let flex = parse_int(&get(&["flex"])).unwrap_or(0);
        Some(base + flex)
    })?;
    if credits == 0 {
        return None;
    }

    let base = parse_int(&get(&["base"]));
    let flex = parse_int(&get(&["flex"]));
    let detail = match (base, flex) {
        (Some(base), Some(flex)) if flex > 0 => {
            format!("${} base + ${} flex", base / 100, flex / 100)
        }
        _ if raw_name.to_ascii_lowercase().contains("business")
            || raw_name.to_ascii_lowercase().contains("enterprise") =>
        {
            format!("{} credits / user, pooled", comma(credits))
        }
        _ => format!("{} credits", comma(credits)),
    };

    Some(Plan {
        name,
        price_usd: if price > 0.0 {
            price
        } else {
            fallback_plan_price(&raw_name)
        },
        credits,
        detail,
    })
}

fn normalize_plan_name(raw: &str) -> Option<String> {
    let key = raw.to_ascii_lowercase();
    if key.contains("pro+") || key.contains("pro plus") {
        Some("Pro+".into())
    } else if key.contains("max") {
        Some("Max".into())
    } else if key.contains("enterprise") {
        Some("Enterprise".into())
    } else if key.contains("business") {
        Some("Business".into())
    } else if key.contains("pro") && !key.contains("product") {
        Some("Pro".into())
    } else {
        None
    }
}

fn fallback_plan_price(raw: &str) -> f64 {
    match normalize_plan_name(raw).as_deref() {
        Some("Pro") => 10.0,
        Some("Pro+") => 39.0,
        Some("Max") => 100.0,
        Some("Business") => 19.0,
        Some("Enterprise") => 39.0,
        _ => 0.0,
    }
}

fn merge_plans(dst: &mut Vec<Plan>, incoming: Vec<Plan>) {
    for plan in incoming {
        if let Some(existing) = dst.iter_mut().find(|p| p.name == plan.name) {
            if plan.credits > 0 {
                existing.credits = plan.credits;
            }
            if plan.price_usd > 0.0 {
                existing.price_usd = plan.price_usd;
            }
            if !plan.detail.is_empty() {
                existing.detail = plan.detail;
            }
        } else {
            dst.push(plan);
        }
    }
}

fn dedupe_model_ids(models: &mut [Model]) {
    let mut seen = std::collections::HashMap::<String, usize>::new();
    for model in models.iter_mut() {
        let count = seen.entry(model.id.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            model.id = format!("{}-{}", model.id, *count);
        }
    }
}

struct HtmlTable {
    aria: Option<String>,
    caption: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn extract_tables(html: &str) -> Vec<HtmlTable> {
    let mut tables = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut search_from = 0;
    while let Some(rel) = lower[search_from..].find("<table") {
        let start = search_from + rel;
        let Some(tag_end) = html[start..].find('>') else {
            break;
        };
        let open_tag = &html[start..start + tag_end + 1];
        let Some(rel_close) = lower[start..].find("</table>") else {
            break;
        };
        let inner = &html[start + tag_end + 1..start + rel_close];
        tables.push(parse_table(open_tag, inner));
        search_from = start + rel_close + "</table>".len();
    }
    tables
}

fn parse_table(open_tag: &str, inner: &str) -> HtmlTable {
    let aria = attr(open_tag, "aria-labelledby");
    let caption = nth_tag_text(inner, "caption")
        .into_iter()
        .next()
        .unwrap_or_default();
    let mut headers = Vec::new();
    if let Some(thead) = tag_inner(inner, "thead")
        && let Some(row) = extract_rows(&thead, "th").into_iter().next()
    {
        headers = row;
    }
    if headers.is_empty()
        && let Some(row) = extract_rows(inner, "th").into_iter().next()
    {
        headers = row;
    }
    let body = tag_inner(inner, "tbody").unwrap_or(inner.to_string());
    let mut rows = extract_rows(&body, "td");
    rows.retain(|row| row.iter().any(|cell| !cell.trim().is_empty()));
    HtmlTable {
        aria,
        caption,
        headers,
        rows,
    }
}

fn extract_rows(html: &str, cell_tag: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut search_from = 0;
    let open = "<tr";
    while let Some(rel) = lower[search_from..].find(open) {
        let start = search_from + rel;
        let Some(rel_close) = lower[start..].find("</tr>") else {
            break;
        };
        let row_html = &html[start..start + rel_close];
        let cells = nth_tag_text(row_html, cell_tag);
        if !cells.is_empty() {
            rows.push(cells);
        }
        search_from = start + rel_close + 5;
    }
    rows
}

fn nth_tag_text(html: &str, tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut search_from = 0;
    while let Some(rel) = lower[search_from..].find(&open) {
        let start = search_from + rel;
        let Some(gt) = html[start..].find('>') else {
            break;
        };
        let content_at = start + gt + 1;
        let Some(rel_close) = lower[content_at..].find(&close) else {
            break;
        };
        out.push(clean_cell(&html[content_at..content_at + rel_close]));
        search_from = content_at + rel_close + close.len();
    }
    out
}

fn tag_inner(html: &str, tag: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = lower.find(&open)?;
    let gt = html[start..].find('>')?;
    let content_at = start + gt + 1;
    let end = lower[content_at..].find(&close)?;
    Some(html[content_at..content_at + end].to_string())
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=");
    let lower = tag.to_ascii_lowercase();
    let idx = lower.find(&needle)?;
    let rest = &tag[idx + needle.len()..];
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let end = rest[1..].find(quote)?;
    Some(rest[1..1 + end].to_string())
}

fn clean_cell(html: &str) -> String {
    let without_sup = strip_element(html, "sup");
    let mut text = collapse_ws(&decode_entities(&strip_tags(&without_sup)));
    for id in footnote_ids_in(html) {
        text.push_str(" [^");
        text.push_str(&id);
        text.push(']');
    }
    text
}

fn footnote_ids_in(html: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let marker = "user-content-fn-";
    let mut from = 0;
    while let Some(rel) = html[from..].find(marker) {
        let start = from + rel + marker.len();
        let rest = &html[start..];
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .unwrap_or(rest.len());
        let id = rest[..end].to_string();
        if !id.is_empty() && !ids.contains(&id) {
            ids.push(id);
        }
        from = start + end;
    }
    ids
}

fn strip_element(html: &str, tag: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    if let Some(start) = lower.find(&open)
        && let Some(end_rel) = lower[start..].find(&close)
    {
        let mut out = String::new();
        out.push_str(&html[..start]);
        out.push_str(&html[start + end_rel + close.len()..]);
        return strip_element(&out, tag);
    }
    html.to_string()
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&le;", "≤")
        .replace("&ge;", "≥")
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_money(raw: &str) -> Option<f64> {
    let t = collapse_ws(raw);
    if t.is_empty()
        || t.eq_ignore_ascii_case("not applicable")
        || t == "—"
        || t == "–"
        || t == "-"
        || t.eq_ignore_ascii_case("n/a")
    {
        return None;
    }
    let t = t.trim_start_matches('$').replace(',', "");
    let t = t.split_whitespace().next().unwrap_or(&t);
    t.parse().ok()
}

fn parse_int(raw: &str) -> Option<u32> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn parse_threshold(raw: &str) -> Option<u32> {
    let t = collapse_ws(raw);
    if t.is_empty() || t.eq_ignore_ascii_case("not applicable") {
        return None;
    }
    let mut num = String::new();
    let mut factor = 1u32;
    for ch in t.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            num.push(ch);
        } else if ch == 'K' || ch == 'k' {
            factor = 1_000;
        } else if ch == 'M' || ch == 'm' {
            factor = 1_000_000;
        }
    }
    if num.is_empty() {
        return None;
    }
    let value: f64 = num.parse().ok()?;
    Some((value * f64::from(factor)) as u32)
}

fn parse_footnotes(html: &str) -> Vec<(String, String)> {
    let mut notes = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut search_from = 0;
    let marker = "id=\"user-content-fn-";
    while let Some(rel) = lower[search_from..].find(marker) {
        let start = search_from + rel + marker.len();
        let Some(end_id) = html[start..].find('"') else {
            break;
        };
        let id = html[start..start + end_id].to_string();
        let after = start + end_id;
        let close = lower[after..]
            .find("</li>")
            .or_else(|| lower[after..].find("</p>"))
            .unwrap_or(200.min(html.len().saturating_sub(after)));
        let text = collapse_ws(&strip_tags(&html[after..after + close]));
        let text = text
            .trim_start_matches(['>', '"', ' '])
            .replace('↩', " ")
            .trim()
            .to_string();
        let text = text
            .trim_end_matches(|c: char| c.is_ascii_digit() || c.is_whitespace())
            .trim()
            .to_string();
        if !text.is_empty() {
            notes.push((id, text));
        }
        search_from = after + close;
    }
    notes
}

fn footnote_for(model_html_or_text: &str, footnotes: &[(String, String)]) -> Option<String> {
    let key = model_html_or_text.to_ascii_lowercase();
    footnotes.iter().find_map(|(id, text)| {
        if key.contains(id) || key.contains(&id.replace('-', "")) {
            Some(text.clone())
        } else {
            None
        }
    })
}

fn comma(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

fn now_stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc(secs)
}

fn format_utc(secs: u64) -> String {
    let z = secs as i64;
    let days = z.div_euclid(86_400);
    let tod = z.rem_euclid(86_400) as u64;
    let (year, month, day) = civil_from_days(days);
    let hour = tod / 3600;
    let minute = (tod % 3600) / 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02} UTC")
}

/// Howard Hinnant's civil-from-days, days since Unix epoch.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
        <h3>OpenAI</h3>
        <table aria-labelledby="openai">
          <thead><tr>
            <th>Model</th><th>Release status</th><th>Category</th>
            <th>Tier</th><th>Threshold (input tokens)</th>
            <th>Input</th><th>Cached input</th><th>Cache write</th><th>Output</th>
          </tr></thead>
          <tbody>
            <tr>
              <td>GPT-5 mini</td><td>GA</td><td>Lightweight</td>
              <td>Default</td><td>Not applicable</td>
              <td>$0.25</td><td>$0.025</td><td>Not applicable</td><td>$2.00</td>
            </tr>
            <tr>
              <td>GPT-5.4</td><td>GA</td><td>Versatile</td>
              <td>Long context</td><td>&gt; 272K</td>
              <td>$5.00</td><td>$0.50</td><td>Not applicable</td><td>$22.50</td>
            </tr>
          </tbody>
        </table>
        <h3>Anthropic</h3>
        <table aria-labelledby="anthropic">
          <thead><tr>
            <th>Model</th><th>Release status</th><th>Category</th>
            <th>Input</th><th>Cached input</th><th>Cache write</th><th>Output</th>
          </tr></thead>
          <tbody>
            <tr>
              <td>Claude Sonnet 5</td><td>GA</td><td>Versatile</td>
              <td>$2.00</td><td>$0.20</td><td>$2.50</td><td>$10.00</td>
            </tr>
            <tr>
              <td>Claude Opus 4.8 (fast mode) (preview)</td><td>GA</td><td>Powerful</td>
              <td>$10.00</td><td>$1.00</td><td>$12.50</td><td>$50.00</td>
            </tr>
          </tbody>
        </table>
        <li id="user-content-fn-gemini-flash-promo"><p>Promotional pricing through December 31, 2026.</p></li>
    "#;

    #[test]
    fn parses_published_table_shape() {
        let models = parse_pricing_tables(SAMPLE).unwrap();
        assert_eq!(models.len(), 4);

        let mini = models.iter().find(|m| m.id == "gpt-5-mini").unwrap();
        assert_eq!(mini.provider, Provider::OpenAI);
        assert_eq!(mini.input, 0.25);
        assert_eq!(mini.cached, 0.025);
        assert_eq!(mini.output, 2.0);
        assert!(mini.cache_write.is_none());

        let long = models.iter().find(|m| m.id == "gpt-5.4-long").unwrap();
        assert_eq!(long.threshold, Some(272_000));
        assert_eq!(long.tier, Tier::LongContext);
        assert_eq!(long.variant, "long >272K");

        let sonnet = models.iter().find(|m| m.id == "claude-sonnet-5").unwrap();
        assert_eq!(sonnet.cache_write, Some(2.50));

        let fast = models
            .iter()
            .find(|m| m.id == "claude-opus-4.8-fast")
            .unwrap();
        assert_eq!(fast.variant, "fast mode");
        assert_eq!(fast.name, "Claude Opus 4.8");
    }

    #[test]
    fn parses_plan_table() {
        let html = r#"
            <table>
              <thead><tr>
                <th>Plan</th><th>Price per month</th><th>Base credits</th>
                <th>Flex allotment</th><th>Total monthly AI credits</th>
              </tr></thead>
              <tbody>
                <tr>
                  <td>Copilot Pro</td><td>$10 USD</td><td>1,000</td><td>500</td><td>1,500</td>
                </tr>
                <tr>
                  <td>Copilot Pro+</td><td>$39 USD</td><td>3,900</td><td>3,100</td><td>7,000</td>
                </tr>
              </tbody>
            </table>
        "#;
        let plans = parse_individual_plans(html);
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].name, "Pro");
        assert_eq!(plans[0].credits, 1_500);
        assert_eq!(plans[0].price_usd, 10.0);
        assert_eq!(plans[1].name, "Pro+");
        assert_eq!(plans[1].credits, 7_000);
    }

    #[test]
    fn money_and_threshold_helpers() {
        assert_eq!(parse_money("$0.075"), Some(0.075));
        assert_eq!(parse_money("Not applicable"), None);
        assert_eq!(parse_threshold("≤ 272K"), Some(272_000));
        assert_eq!(parse_threshold("> 200K"), Some(200_000));
        assert_eq!(parse_threshold("Not applicable"), None);
    }

    #[test]
    fn utc_stamp_round_trip_known_instant() {
        assert_eq!(format_utc(0), "1970-01-01 00:00 UTC");
        assert_eq!(format_utc(1_704_067_200), "2024-01-01 00:00 UTC");
    }

    #[test]
    fn org_parser_skips_promotional_table() {
        let html = r#"
            <table>
              <thead><tr><th>Plan</th><th>Total AI credits per user per month</th></tr></thead>
              <tbody>
                <tr><td>Copilot Business</td><td>1,900</td></tr>
                <tr><td>Copilot Enterprise</td><td>3,900</td></tr>
              </tbody>
            </table>
            <h3>Promotional amounts for existing customers</h3>
            <table>
              <thead><tr><th>Plan</th><th>Total AI credits per user per month</th></tr></thead>
              <tbody>
                <tr><td>Copilot Business</td><td>3,000</td></tr>
                <tr><td>Copilot Enterprise</td><td>7,000</td></tr>
              </tbody>
            </table>
        "#;
        let plans = parse_org_plans(html);
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].name, "Business");
        assert_eq!(plans[0].credits, 1_900);
        assert_eq!(plans[1].credits, 3_900);
    }

    #[test]
    fn parses_vals_overall_accuracy_first_hit() {
        let html = r#"
            &quot;openai/gpt-5.6-sol&quot;:[0,{&quot;accuracy&quot;:[0,96.2],&quot;latency&quot;:[0,182]}]
            &quot;anthropic/claude-opus-5&quot;:[0,{&quot;accuracy&quot;:[0,97],&quot;latency&quot;:[0,100]}]
            &quot;openai/gpt-5.6-sol&quot;:[0,{&quot;accuracy&quot;:[0,100],&quot;latency&quot;:[0,1]}]
        "#;
        let scores = parse_vals_accuracies(html);
        assert_eq!(scores.len(), 2);
        assert_eq!(scores[0], ("openai/gpt-5.6-sol".into(), 96.2));
        assert_eq!(scores[1], ("anthropic/claude-opus-5".into(), 97.0));
    }

    #[test]
    fn extract_promo_pulls_through_date() {
        let note = "Gemini 3.6 Flash and Gemini 3.7 Flash are available at the promotional pricing of $0.75 per 1M input tokens through December 31, 2026.";
        assert_eq!(
            extract_promo(note).as_deref(),
            Some("through December 31, 2026")
        );
        assert!(extract_promo("standard GA pricing").is_none());
    }
}
