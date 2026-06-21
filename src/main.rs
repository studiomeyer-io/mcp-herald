//! `mcp-herald` — a thin CLI over the [`mcp_herald`] library.
//!
//! ```text
//! mcp-herald lint ./src                      # scan a tree for 2026-07-28 issues
//! mcp-herald lint ./src --fail-on warning    # gate CI
//! mcp-herald lint ./src --format sarif        # GitHub code scanning
//! mcp-herald rules                            # show the ruleset + sources
//! ```

use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, Parser, Subcommand, ValueEnum};

use mcp_herald::report::{render, render_rules};
use mcp_herald::rules::Level;
use mcp_herald::{compile, sarif, scan_path};

#[derive(Parser, Debug)]
#[command(
    name = "mcp-herald",
    version,
    about = "Static migration linter for the MCP 2026-07-28 spec."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Scan a file or directory for 2026-07-28 migration issues.
    Lint(LintArgs),
    /// Print the ruleset with its SEP/RFC sources.
    Rules,
}

#[derive(Args, Debug)]
struct LintArgs {
    /// Path to scan (file or directory).
    #[arg(default_value = ".", value_name = "PATH")]
    path: PathBuf,
    /// Exit non-zero when a finding at or above this level is present.
    #[arg(long, value_enum, default_value_t = FailOn::Error)]
    fail_on: FailOn,
    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Human)]
    format: Format,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Format {
    Human,
    Sarif,
    Json,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum FailOn {
    Error,
    Warning,
    Info,
    Never,
}

fn main() {
    match run(Cli::parse()) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(2);
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<i32> {
    match cli.cmd {
        Cmd::Rules => {
            print!("{}", render_rules());
            Ok(0)
        }
        Cmd::Lint(a) => lint(a),
    }
}

fn lint(a: LintArgs) -> anyhow::Result<i32> {
    if !a.path.exists() {
        anyhow::bail!("path does not exist: {}", a.path.display());
    }
    let rules = compile();
    let report = scan_path(&a.path, &rules);
    let root = a.path.display().to_string();

    match a.format {
        Format::Human => print!("{}", render(&report, &root)),
        Format::Sarif => {
            let s = sarif::to_sarif(&report);
            println!(
                "{}",
                serde_json::to_string_pretty(&s).context("serializing SARIF")?
            );
        }
        Format::Json => println!(
            "{}",
            serde_json::to_string_pretty(&report).context("serializing JSON")?
        ),
    }

    Ok(exit_code(&report, a.fail_on))
}

fn exit_code(report: &mcp_herald::Report, fail_on: FailOn) -> i32 {
    let threshold = match fail_on {
        FailOn::Error => Level::Error,
        FailOn::Warning => Level::Warning,
        FailOn::Info => Level::Info,
        FailOn::Never => return 0,
    };
    if report.has_at_least(threshold) {
        1
    } else {
        0
    }
}
