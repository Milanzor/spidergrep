# spidergrep

Crawls all pages on a domain and reports every URL where a pattern (regex) is found.

## Install

### Pre-built binary

Download the latest binary for your platform from the [releases page](../../releases), make it executable, and move it to your PATH:

```sh
chmod +x spidergrep
mv spidergrep ~/.local/bin/
```

### Build from source

Requires [Rust](https://rustup.rs).

```sh
git clone <repo-url>
cd spidergrep
cargo install --path .
```

## Usage

```
spidergrep <URL> <PATTERN> [OPTIONS]
```

```
Options:
  -A, --user-agent <UA>    custom User-Agent string
  -d, --delay <MS>         delay between requests in milliseconds [default: 0]
  -j, --concurrency <N>    parallel requests [default: 4]
      --max-depth <N>      crawl depth limit, 0 = unlimited [default: 0]
      --max-urls <N>       max pages to crawl, 0 = unlimited [default: 0]
      --timeout <SECS>     per-request timeout [default: 30]
  -v, -vv, -vvv            verbosity (pages skipped / links found / HTTP details)
  -q, --quiet              only print matches
  -s, --case-sensitive     matching is case-insensitive by default
  -C, --context <N>        lines of context around each match
      --insecure           ignore TLS certificate errors
  -o, --output <FILE>      write results to a file
  -h, --help               print help
```

**Examples**

```sh
# Find all pages mentioning "contact"
spidergrep https://example.com contact

# Regex, case-sensitive, with 2 lines of context
spidergrep -s -C 2 https://example.com "api[_-]key"

# Slow down to avoid hammering the server
spidergrep --delay 500 -j 1 https://example.com "todo"

# Save results to a file
spidergrep https://example.com "price" -o results.txt
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0    | One or more matches found |
| 1    | No matches found |
| 2    | Error (bad URL, invalid regex, network failure, …) |
