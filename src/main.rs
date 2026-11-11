use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None => match copilot_pricing::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("error: {err}");
                ExitCode::FAILURE
            }
        },
        Some("update") | Some("--update") => match update() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("error: {err}");
                ExitCode::FAILURE
            }
        },
        Some("-h") | Some("--help") | Some("help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            print_help();
            ExitCode::FAILURE
        }
    }
}

fn update() -> Result<(), String> {
    let (catalog, path) = copilot_pricing::fetch::refresh_and_save()?;
    println!(
        "Wrote {} models, {} plans, {} SWE scores ({} models matched) to {}",
        catalog.models.len(),
        catalog.plans.len(),
        catalog.benchmarks.scores.len(),
        catalog.scored_count(),
        path.display()
    );
    println!("Source: {}", catalog.source);
    Ok(())
}

fn print_help() {
    let mut out = io::stdout();
    let _ = writeln!(
        out,
        "\
copilot-pricing — compare GitHub Copilot model rates

Usage:
  copilot-pricing           Launch the TUI
  copilot-pricing update    Fetch the official docs page and rewrite data/pricing.json
  copilot-pricing --help    Show this help

The TUI reads data/pricing.json (or $COPILOT_PRICING_DATA). Press u in the
app to refresh the same file from:
  https://docs.github.com/en/copilot/reference/copilot-billing/models-and-pricing"
    );
}
