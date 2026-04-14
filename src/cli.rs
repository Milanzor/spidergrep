use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "spidergrep",
    about = "Spider a website and grep for a pattern",
    long_about = "Crawls all pages on a domain starting from ENTRY_URL and reports\n\
                  every URL where PATTERN (a regex) is found in the page content."
)]
pub struct Args {
    /// Entry URL to start crawling from
    #[arg(required_unless_present = "update")]
    pub url: Option<String>,

    /// Pattern to search for (regex supported)
    #[arg(required_unless_present = "update")]
    pub pattern: Option<String>,

    /// Custom User-Agent header
    #[arg(short = 'A', long, value_name = "UA")]
    pub user_agent: Option<String>,

    /// Delay between requests in milliseconds
    #[arg(short, long, default_value_t = 0, value_name = "MS")]
    pub delay: u64,

    /// Maximum crawl depth from the entry URL (0 = unlimited)
    #[arg(long, default_value_t = 0, value_name = "N")]
    pub max_depth: usize,

    /// Maximum number of URLs to crawl (0 = unlimited)
    #[arg(long, default_value_t = 0, value_name = "N")]
    pub max_urls: usize,

    /// Request timeout in seconds
    #[arg(long, default_value_t = 30, value_name = "SECS")]
    pub timeout: u64,

    /// Verbosity level (use multiple times: -v, -vv, -vvv)
    ///
    /// -v   : show pages with no match
    /// -vv  : also show extracted links per page
    /// -vvv : also show HTTP details
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Quiet mode — only print match results, no progress info
    #[arg(short, long, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Case-sensitive pattern matching (matching is case-insensitive by default)
    #[arg(short = 's', long)]
    pub case_sensitive: bool,

    /// Show N lines of context around each match (like grep -C)
    #[arg(short = 'C', long, default_value_t = 0, value_name = "N")]
    pub context: usize,

    /// Accept invalid/self-signed TLS certificates
    #[arg(long)]
    pub insecure: bool,

    /// Write match results to FILE (in addition to stdout)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<String>,

    /// Maximum concurrent requests
    #[arg(short = 'j', long, default_value_t = 4, value_name = "N")]
    pub concurrency: usize,

    /// Check for a newer release and update the binary in place
    #[arg(long)]
    pub update: bool,
}
