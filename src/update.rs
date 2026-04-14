use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::Write;

const REPO: &str = "Milanzor/spidergrep";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

/// Returns the target triple that matches the running binary's platform.
fn current_target() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Some("x86_64-unknown-linux-musl");

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Some("aarch64-unknown-linux-musl");

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Some("x86_64-apple-darwin");

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Some("aarch64-apple-darwin");

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Some("x86_64-pc-windows-msvc");

    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    return None;
}

/// Compare two semver-ish strings (strips leading `v`).
/// Returns true if `latest` is strictly newer than `current`.
fn is_newer(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.trim_start_matches('v')
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    parse(latest) > parse(current)
}

pub async fn run() -> Result<()> {
    let target = current_target().context(
        "Unsupported platform — pre-built binaries are only available for \
         Linux (x86_64/aarch64), macOS (x86_64/aarch64), and Windows (x86_64).",
    )?;

    println!("Current version : {}", CURRENT_VERSION.yellow());
    println!("Checking        : https://github.com/{REPO}/releases/latest");

    let client = reqwest::Client::builder()
        .user_agent(format!("spidergrep/{CURRENT_VERSION}"))
        .build()?;

    let release: Release = client
        .get(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .send()
        .await
        .context("Reaching GitHub API")?
        .json()
        .await
        .context("Parsing GitHub API response")?;

    let latest = &release.tag_name;

    if !is_newer(CURRENT_VERSION, latest) {
        println!("Latest version  : {}", latest.green());
        println!("{}", "Already up to date.".green().bold());
        return Ok(());
    }

    println!("Latest version  : {}", latest.yellow());
    println!("Updating {} → {}...", CURRENT_VERSION.dimmed(), latest.green().bold());

    // Build the download URL.
    let (archive_name, is_zip) = if target.contains("windows") {
        (format!("spidergrep-{latest}-{target}.zip"), true)
    } else {
        (format!("spidergrep-{latest}-{target}.tar.gz"), false)
    };

    let url = format!("https://github.com/{REPO}/releases/download/{latest}/{archive_name}");

    // Download archive into memory.
    let bytes = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Downloading {url}"))?
        .bytes()
        .await
        .context("Reading download body")?;

    // Extract the binary from the archive.
    let binary_bytes = if is_zip {
        extract_from_zip(&bytes)?
    } else {
        extract_from_targz(&bytes)?
    };

    // Replace the current executable atomically.
    let current_exe = env::current_exe().context("Locating current executable")?;
    let tmp_path = current_exe.with_extension("tmp");

    {
        let mut tmp = fs::File::create(&tmp_path)
            .with_context(|| format!("Creating {}", tmp_path.display()))?;
        tmp.write_all(&binary_bytes)
            .context("Writing new binary")?;
    }

    // Make executable on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755))?;
    }

    fs::rename(&tmp_path, &current_exe).with_context(|| {
        format!(
            "Replacing {} (you may need to re-run with sudo)",
            current_exe.display()
        )
    })?;

    println!(
        "{} updated to {}",
        "spidergrep".cyan().bold(),
        latest.green().bold()
    );

    Ok(())
}

fn extract_from_targz(bytes: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    use tar::Archive;

    let gz = GzDecoder::new(bytes);
    let mut archive = Archive::new(gz);

    for entry in archive.entries().context("Reading tar entries")? {
        let mut entry = entry.context("Reading tar entry")?;
        let path = entry.path().context("Reading entry path")?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name == "spidergrep" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).context("Reading binary from tar")?;
            return Ok(buf);
        }
    }

    bail!("Could not find `spidergrep` binary inside the downloaded archive");
}

fn extract_from_zip(bytes: &[u8]) -> Result<Vec<u8>> {
    use std::io::{Cursor, Read};

    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("Opening zip archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("Reading zip entry")?;
        let name = file.name().to_string();
        if name == "spidergrep.exe" || name == "spidergrep" {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).context("Reading binary from zip")?;
            return Ok(buf);
        }
    }

    bail!("Could not find `spidergrep.exe` inside the downloaded archive");
}
