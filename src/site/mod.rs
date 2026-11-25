//! Static site export for GitHub Pages (sourcecritic.freedxm.com).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
    if let Ok(games) = db.list_games() {
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

    let lastmod = chrono_date();
    let sitemap = build_sitemap(&files, &lastmod);
    fs::write(out_dir.join("sitemap.xml"), &sitemap)?;
    let images = build_image_sitemap(&files, &lastmod);
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

fn build_sitemap(files: &[(String, String)], lastmod: &str) -> String {
    let mut urls: Vec<String> = files
        .iter()
        .filter_map(|(path, html)| {
            if html.starts_with("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><link rel=\"canonical\"")
                && html.contains("http-equiv=\"refresh\"")
            {
                return None;
            }
            let loc = page_loc(path)?;
            Some(format!(
                "  <url><loc>{loc}</loc><lastmod>{lastmod}</lastmod><changefreq>weekly</changefreq></url>"
            ))
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

fn build_image_sitemap(files: &[(String, String)], lastmod: &str) -> String {
    let mut urls = Vec::new();
    for (path, html) in files {
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
