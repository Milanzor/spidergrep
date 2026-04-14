mod cli;
mod fetcher;
mod spider;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use regex::RegexBuilder;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;

use cli::Args;
use fetcher::HttpFetcher;
use spider::{extract_base_host, Spider, SpiderResult};

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (compatible; spidergrep/0.1; +https://github.com/spidergrep)";

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let pattern = args.pattern.clone();
    let regex = RegexBuilder::new(&pattern)
        .case_insensitive(!args.case_sensitive)
        .build()
        .map_err(|e| anyhow::anyhow!("Invalid regex pattern `{pattern}`: {e}"))?;

    let user_agent = args
        .user_agent
        .as_deref()
        .unwrap_or(DEFAULT_USER_AGENT)
        .to_string();

    let base_host = extract_base_host(&args.url)?;

    let quiet = args.quiet;
    let verbose = args.verbose;
    let output_path = args.output.clone();

    if !quiet {
        eprintln!(
            "{} {} for {} on {}",
            "spidergrep".bold().cyan(),
            "starting".dimmed(),
            pattern.yellow(),
            args.url.cyan()
        );
        if verbose >= 3 {
            eprintln!("  user-agent : {user_agent}");
            eprintln!("  delay      : {}ms", args.delay);
            eprintln!("  timeout    : {}s", args.timeout);
            eprintln!("  concurrency: {}", args.concurrency);
        }
        eprintln!();
    }

    let fetcher = Arc::new(HttpFetcher::new(&user_agent, args.timeout, args.insecure)?);

    let spider = Spider::new(fetcher, regex, args, base_host);

    let SpiderResult {
        matches,
        pages_scanned,
        pages_skipped,
    } = spider.run().await?;

    if let Some(ref path) = output_path {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        for m in &matches {
            writeln!(file, "{}:{}:{}", m.url, m.line_number, m.line)?;
        }
    }

    if !quiet {
        eprintln!();
        eprintln!(
            "{} {} match{} across {} page{}{}",
            "done:".bold(),
            matches.len().to_string().bold().green(),
            if matches.len() == 1 { "" } else { "es" },
            pages_scanned.to_string().bold(),
            if pages_scanned == 1 { "" } else { "s" },
            if pages_skipped > 0 {
                format!(" ({pages_skipped} failed)").dimmed().to_string()
            } else {
                String::new()
            }
        );
    }

    if matches.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}
