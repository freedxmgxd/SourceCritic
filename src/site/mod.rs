//! Static site export for GitHub Pages (sourcecritic.freedxm.com).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::api::pages;
use crate::db::Database;

const SITE_ORIGIN: &str = "https://sourcecritic.freedxm.com";
const SITE_HOST: &str = "sourcecritic.freedxm.com";

#[derive(Debug)]
pub enum ExportError {
    Io(io::Error),
    Db(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Io(error) => write!(f, "{error}"),
            ExportError::Db(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<io::Error> for ExportError {
    fn from(error: io::Error) -> Self {
        ExportError::Io(error)
    }
}

/// Write a full static site under `out_dir` (HTML + covers + GitHub Pages files).
pub fn export_static_site(db: &Database, out_dir: &Path) -> Result<ExportStats, ExportError> {
    let dupes = db.duplicate_review_pairs().map_err(|error| ExportError::Db(error.to_string()))?;
    if !dupes.is_empty() {
        let sample = dupes.iter().take(20).cloned().collect::<Vec<_>>().join("; ");
        return Err(ExportError::Db(format!(
            "{} duplicate (outlet, URL) pairs ({sample})",
            dupes.len()
        )));
    }
    if out_dir.exists() {
        eprintln!("removing {}", out_dir.display());
        fs::remove_dir_all(out_dir)?;
    }
    fs::create_dir_all(out_dir)?;
    eprintln!("rendering pages");

    let files = pages::render_all_pages(db).map_err(ExportError::Db)?;
    pages::audit_pages(&files).map_err(ExportError::Db)?;
    eprintln!("writing {} pages", files.len());
    let mut html_pages = 0usize;
    let mut redirects: Vec<(String, String)> = Vec::new();
    for (rel, html) in &files {
        if rel.starts_with("best/") {
            if let Some(target) = pages::refresh_target(html) {
                redirects.push((static_url(rel), target));
            }
        }
        let path = out_dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, html)?;
        html_pages += 1;
    }
    if !redirects.is_empty() {
        let mut body = String::from("# Legacy /best/{platform}/{year}/ URLs\n");
        for (from, to) in &redirects {
            body.push_str(&format!("{from} {to} 301\n"));
        }
        fs::write(out_dir.join("_redirects"), body)?;
    }

    let covers_src = PathBuf::from("static/covers");
    let covers_dst = out_dir.join("covers");
    let mut cover_files = 0usize;
    if covers_src.is_dir() {
        cover_files = copy_dir_recursive(&covers_src, &covers_dst)?;
    }
    for name in ["favicon.ico", "favicon.svg", "apple-touch-icon.png", "og-default.png"] {
        let src = PathBuf::from("static").join(name);
        if src.is_file() {
            fs::copy(&src, out_dir.join(name))?;
        }
    }
    if let Ok(mut games) = db.list_games() {
        games.retain(|game| pages::withhold_page(game).is_none());
        fs::write(out_dir.join("games-index.json"), pages::games_index_json(&games))?;
    }
    let og_dir = out_dir.join("og");
    let status = std::process::Command::new("python3")
        .args([
            "scripts/og_cards.py",
            &og_dir.to_string_lossy(),
            "sourcecritic.db",
        ])
        .status()?;
    if !status.success() {
        return Err(ExportError::Db("og card generation failed".into()));
    }

    // GitHub Pages: custom domain + disable Jekyll (keeps folders like covers intact).
    fs::write(out_dir.join("CNAME"), format!("{SITE_HOST}\n"))?;
    fs::write(out_dir.join(".nojekyll"), "")?;

    fs::write(
        out_dir.join("robots.txt"),
        format!(
            "User-agent: *\nAllow: /\nSitemap: {SITE_ORIGIN}/sitemap.xml\nSitemap: {SITE_ORIGIN}/sitemap-images.xml\n"
        ),
    )?;

    // AdSense authorized sellers (same pub as freedxm.com).
    fs::write(
        out_dir.join("ads.txt"),
        "google.com, pub-5490722539415533, DIRECT, f08c47fec0942fa0\n",
    )?;

    let today = chrono_date();
    let stamps = load_stamps(Path::new("data/lastmod.json"));
    let mut next = BTreeMap::new();
    for (path, html) in &files {
        if pages::refresh_target(html).is_some() {
            continue;
        }
        let Some(loc) = page_loc(path) else {
            continue;
        };
        let hash = content_hash(html);
        let lastmod = match stamps.get(&loc) {
            Some(stamp) if stamp.hash == hash => stamp.lastmod.clone(),
            _ => today.clone(),
        };
        next.insert(loc, Stamp { hash, lastmod });
    }
    if let Some(parent) = Path::new("data/lastmod.json").parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(&next).map_err(|error| ExportError::Db(error.to_string()))?;
    fs::write("data/lastmod.json", format!("{body}\n"))?;
    let sitemap = build_sitemap(&next);
    fs::write(out_dir.join("sitemap.xml"), &sitemap)?;
    let images = build_image_sitemap(&files, &next);
    fs::write(out_dir.join("sitemap-images.xml"), images)?;

    Ok(ExportStats {
        html_pages,
        cover_files,
        out_dir: out_dir.to_path_buf(),
    })
}

#[derive(Debug)]
pub struct ExportStats {
    pub html_pages: usize,
    pub cover_files: usize,
    pub out_dir: PathBuf,
}

fn chrono_date() -> String {
    // Build day, UTC. Avoid a time crate for a static export stamp.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn static_url(rel: &str) -> String {
    if let Some(dir) = rel.strip_suffix("/index.html") {
        format!("/{dir}/")
    } else if rel == "index.html" {
        "/".to_string()
    } else {
        format!("/{rel}")
    }
}

fn page_loc(path: &str) -> Option<String> {
    if path == "404.html" || path.contains("redirect") {
        return None;
    }
    if path == "index.html" {
        return Some(format!("{SITE_ORIGIN}/"));
    }
    path.strip_suffix("/index.html")
        .map(|dir| format!("{SITE_ORIGIN}/{dir}/"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Stamp {
    hash: String,
    lastmod: String,
}

fn load_stamps(path: &Path) -> BTreeMap<String, Stamp> {
    let Ok(bytes) = fs::read(path) else {
        return BTreeMap::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn content_hash(html: &str) -> String {
    fn fnv(bytes: &[u8], offset: u64) -> u64 {
        let mut hash = offset;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }
    let bytes = html.as_bytes();
    format!(
        "{:016x}{:016x}",
        fnv(bytes, 0xcbf2_9ce4_8422_2325),
        fnv(bytes, 0x8422_2325_cbf2_9ce4)
    )
}

fn build_sitemap(stamps: &BTreeMap<String, Stamp>) -> String {
    let mut urls: Vec<String> = stamps
        .iter()
        .map(|(loc, stamp)| {
            format!(
                "  <url><loc>{loc}</loc><lastmod>{}</lastmod><changefreq>weekly</changefreq></url>",
                stamp.lastmod
            )
        })
        .collect();
    urls.sort();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         {}\n\
         </urlset>\n",
        urls.join("\n")
    )
}

fn build_image_sitemap(files: &[(String, String)], stamps: &BTreeMap<String, Stamp>) -> String {
    let mut urls = Vec::new();
    for (path, html) in files {
        if pages::refresh_target(html).is_some() {
            continue;
        }
        let Some(loc) = page_loc(path) else { continue };
        if !path.starts_with("game/") {
            continue;
        }
        let Some(src) = html.split("property=\"og:image\" content=\"").nth(1) else {
            continue;
        };
        let Some(image) = src.split('"').next() else { continue };
        if image.is_empty() || image.ends_with("/og-default.png") {
            continue;
        }
        let lastmod = stamps
            .get(&loc)
            .map(|stamp| stamp.lastmod.as_str())
            .unwrap_or("");
        urls.push(format!(
            "  <url><loc>{loc}</loc><lastmod>{lastmod}</lastmod>\
             <image:image><image:loc>{image}</image:loc></image:image></url>"
        ));
    }
    urls.sort();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" \
         xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\">\n\
         {}\n\
         </urlset>\n",
        urls.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unchanged_hash_keeps_its_date() {
        let mut previous = BTreeMap::new();
        previous.insert(
            "https://sourcecritic.freedxm.com/game/hades/".to_string(),
            Stamp {
                hash: content_hash("<p>same</p>"),
                lastmod: "2020-01-02".into(),
            },
        );
        let hash = content_hash("<p>same</p>");
        let kept = match previous.get("https://sourcecritic.freedxm.com/game/hades/") {
            Some(stamp) if stamp.hash == hash => stamp.lastmod.clone(),
            _ => "2026-11-26".into(),
        };
        assert_eq!(kept, "2020-01-02");
        let changed = content_hash("<p>edited</p>");
        let bumped = match previous.get("https://sourcecritic.freedxm.com/game/hades/") {
            Some(stamp) if stamp.hash == changed => stamp.lastmod.clone(),
            _ => "2026-11-26".into(),
        };
        assert_eq!(bumped, "2026-11-26");
        assert_ne!(content_hash("<p>same</p>"), content_hash("<p>edited</p>"));
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<usize> {
    fs::create_dir_all(dst)?;
    let mut count = 0usize;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            count += copy_dir_recursive(&entry.path(), &to)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), &to)?;
            count += 1;
        }
    }
    Ok(count)
}
