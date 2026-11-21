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
        fs::remove_dir_all(out_dir)?;
    }
    fs::create_dir_all(out_dir)?;

    let files = pages::render_all_pages(db).map_err(ExportError::Db)?;
    let mut html_pages = 0usize;
    for (rel, html) in &files {
        let path = out_dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, html)?;
        html_pages += 1;
    }

    let covers_src = PathBuf::from("static/covers");
    let covers_dst = out_dir.join("covers");
    let mut cover_files = 0usize;
    if covers_src.is_dir() {
        cover_files = copy_dir_recursive(&covers_src, &covers_dst)?;
    }

    // GitHub Pages: custom domain + disable Jekyll (keeps folders like covers intact).
    fs::write(out_dir.join("CNAME"), format!("{SITE_HOST}\n"))?;
    fs::write(out_dir.join(".nojekyll"), "")?;

    fs::write(
        out_dir.join("robots.txt"),
        format!("User-agent: *\nAllow: /\nSitemap: {SITE_ORIGIN}/sitemap.xml\n"),
    )?;

    // AdSense authorized sellers (same pub as freedxm.com).
    fs::write(
        out_dir.join("ads.txt"),
        "google.com, pub-5490722539415533, DIRECT, f08c47fec0942fa0\n",
    )?;

    let sitemap = build_sitemap(&files);
    fs::write(out_dir.join("sitemap.xml"), sitemap)?;

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

fn build_sitemap(files: &[(String, String)]) -> String {
    let mut urls: Vec<String> = files
        .iter()
        .filter_map(|(path, _)| {
            if path == "404.html" {
                return None;
            }
            let loc = if path == "index.html" {
                format!("{SITE_ORIGIN}/")
            } else if let Some(dir) = path.strip_suffix("/index.html") {
                format!("{SITE_ORIGIN}/{dir}/")
            } else {
                return None;
            };
            Some(format!(
                "  <url><loc>{loc}</loc><changefreq>weekly</changefreq></url>"
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
