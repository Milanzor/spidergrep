use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use url::Url;

use crate::cli::Args;
use crate::fetcher::{Fetcher, PageContent};

pub struct Match {
    pub url: String,
    pub line_number: usize,
    pub line: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

pub struct SpiderResult {
    pub matches: Vec<Match>,
    pub pages_scanned: usize,
    pub pages_skipped: usize,
}

pub struct Spider {
    fetcher: Arc<dyn Fetcher>,
    regex: Regex,
    args: Args,
    base_host: String,
}

impl Spider {
    pub fn new(fetcher: Arc<dyn Fetcher>, regex: Regex, args: Args, base_host: String) -> Self {
        Self {
            fetcher,
            regex,
            args,
            base_host,
        }
    }

    pub async fn run(&self) -> Result<SpiderResult> {
        let mut queue: VecDeque<(String, usize)> = VecDeque::new(); // (url, depth)
        let mut visited: HashSet<String> = HashSet::new();
        let mut all_matches: Vec<Match> = Vec::new();
        let mut pages_scanned = 0usize;
        let mut pages_skipped = 0usize;

        // Normalise and enqueue the entry URL.
        let entry = normalise_url(self.args.url.as_deref().unwrap_or_default())?;
        queue.push_back((entry.clone(), 0));
        visited.insert(entry);

        let progress = if !self.args.quiet {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::with_template("{spinner:.cyan} {msg}")
                    .unwrap()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            pb.enable_steady_tick(Duration::from_millis(80));
            Some(pb)
        } else {
            None
        };

        let semaphore = Arc::new(Semaphore::new(self.args.concurrency));

        while let Some((url, depth)) = queue.pop_front() {
            // Enforce limits.
            if self.args.max_urls > 0 && pages_scanned >= self.args.max_urls {
                break;
            }

            let queue_len = queue.len();

            if let Some(pb) = &progress {
                pb.set_message(format!(
                    "Scanning {} ({} queued)",
                    url.cyan(),
                    queue_len
                ));
            }

            let _permit = semaphore.acquire().await?;

            match self.fetcher.fetch(&url).await {
                Err(e) => {
                    pages_skipped += 1;
                    if self.args.verbose >= 1 && !self.args.quiet {
                        if let Some(pb) = &progress {
                            pb.suspend(|| {
                                eprintln!("{} {} — {}", "SKIP".yellow(), url, e);
                            });
                        }
                    }
                }
                Ok(PageContent { html, final_url }) => {
                    pages_scanned += 1;

                    // Search the page content.
                    let matches = self.grep_html(&url, &final_url, &html);
                    let found = !matches.is_empty();

                    for m in &matches {
                        self.print_match(m, &progress);
                    }
                    all_matches.extend(matches);

                    // Verbose: show pages with no match.
                    if self.args.verbose >= 1 && !found && !self.args.quiet {
                        if let Some(pb) = &progress {
                            pb.suspend(|| {
                                eprintln!("{} {}", "    ".dimmed(), url.dimmed());
                            });
                        }
                    }

                    // Extract and enqueue links.
                    let next_depth = depth + 1;
                    let skip_depth = self.args.max_depth > 0 && next_depth > self.args.max_depth;

                    if !skip_depth {
                        let links = extract_links(&html, &final_url);
                        if self.args.verbose >= 2 && !self.args.quiet {
                            if let Some(pb) = &progress {
                                pb.suspend(|| {
                                    eprintln!(
                                        "  {} found {} links on {}",
                                        "→".dimmed(),
                                        links.len(),
                                        url
                                    );
                                });
                            }
                        }
                        for link in links {
                            if visited.contains(&link) {
                                continue;
                            }
                            if !self.is_same_domain(&link) {
                                if self.args.verbose >= 2 && !self.args.quiet {
                                    if let Some(pb) = &progress {
                                        pb.suspend(|| {
                                            eprintln!(
                                                "  {} skip external {}",
                                                "·".dimmed(),
                                                link.dimmed()
                                            );
                                        });
                                    }
                                }
                                continue;
                            }
                            visited.insert(link.clone());
                            queue.push_back((link, next_depth));
                        }
                    }
                }
            }

            // Delay between requests.
            if self.args.delay > 0 && !queue.is_empty() {
                tokio::time::sleep(Duration::from_millis(self.args.delay)).await;
            }
        }

        if let Some(pb) = &progress {
            pb.finish_and_clear();
        }

        Ok(SpiderResult {
            matches: all_matches,
            pages_scanned,
            pages_skipped,
        })
    }

    fn grep_html(&self, original_url: &str, _final_url: &str, html: &str) -> Vec<Match> {
        // Search raw HTML line by line — covers attributes, comments, inline scripts, etc.
        let lines: Vec<&str> = html.lines().collect();
        let mut matches = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            if self.regex.is_match(line) {
                let context_before = if self.args.context > 0 {
                    let start = idx.saturating_sub(self.args.context);
                    lines[start..idx]
                        .iter()
                        .map(|l| l.to_string())
                        .collect()
                } else {
                    vec![]
                };
                let context_after = if self.args.context > 0 {
                    let end = (idx + self.args.context + 1).min(lines.len());
                    lines[idx + 1..end]
                        .iter()
                        .map(|l| l.to_string())
                        .collect()
                } else {
                    vec![]
                };
                matches.push(Match {
                    url: original_url.to_string(),
                    line_number: idx + 1,
                    line: line.to_string(),
                    context_before,
                    context_after,
                });
            }
        }

        matches
    }

    fn print_match(&self, m: &Match, progress: &Option<ProgressBar>) {
        let print = || {
            // Context before
            for (i, ctx) in m.context_before.iter().enumerate() {
                let line_no = m.line_number - m.context_before.len() + i;
                println!(
                    "{}{}{}{}",
                    m.url.cyan(),
                    ":".dimmed(),
                    format!("{line_no}-").dimmed(),
                    ctx.dimmed()
                );
            }
            // The match line
            println!(
                "{}{}{}{}",
                m.url.cyan(),
                ":".dimmed(),
                format!("{}:", m.line_number).yellow(),
                highlight_match(&m.line, &self.regex)
            );
            // Context after
            for (i, ctx) in m.context_after.iter().enumerate() {
                let line_no = m.line_number + i + 1;
                println!(
                    "{}{}{}{}",
                    m.url.cyan(),
                    ":".dimmed(),
                    format!("{line_no}-").dimmed(),
                    ctx.dimmed()
                );
            }
        };

        if let Some(pb) = progress {
            pb.suspend(print);
        } else {
            print();
        }
    }

    fn is_same_domain(&self, url: &str) -> bool {
        match Url::parse(url) {
            Ok(u) => u.host_str().map_or(false, |h| h == self.base_host),
            Err(_) => false,
        }
    }
}

fn highlight_match(line: &str, re: &Regex) -> String {
    let mut result = String::new();
    let mut last = 0;
    for mat in re.find_iter(line) {
        result.push_str(&line[last..mat.start()]);
        result.push_str(&line[mat.start()..mat.end()].red().bold().to_string());
        last = mat.end();
    }
    result.push_str(&line[last..]);
    result
}

fn extract_links(html: &str, base_url: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").unwrap();
    let base = match Url::parse(base_url) {
        Ok(u) => u,
        Err(_) => return vec![],
    };

    document
        .select(&selector)
        .filter_map(|el| el.value().attr("href"))
        .filter_map(|href| base.join(href).ok())
        .map(|mut u| {
            // Drop fragment — different fragments on the same page are the same resource.
            u.set_fragment(None);
            u.to_string()
        })
        .filter(|u| u.starts_with("http://") || u.starts_with("https://"))
        .collect()
}

fn normalise_url(raw: &str) -> Result<String> {
    let url = if raw.starts_with("http://") || raw.starts_with("https://") {
        Url::parse(raw)?
    } else {
        Url::parse(&format!("https://{raw}"))?
    };
    Ok(url.to_string())
}

pub fn extract_base_host(raw: &str) -> Result<String> {
    let url = if raw.starts_with("http://") || raw.starts_with("https://") {
        Url::parse(raw)?
    } else {
        Url::parse(&format!("https://{raw}"))?
    };
    url.host_str()
        .map(|h| h.to_string())
        .ok_or_else(|| anyhow::anyhow!("No host in URL: {raw}"))
}
