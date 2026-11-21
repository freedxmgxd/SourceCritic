use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use sourcecritic::db::Database;
use sourcecritic::site;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let out = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("dist"));
    let db_path = env::var("SOURCECRITIC_DB").unwrap_or_else(|_| "sourcecritic.db".to_string());

    let db = match Database::open(&db_path) {
        Ok(db) => db,
        Err(error) => {
            eprintln!("failed to open {db_path}: {error}");
            return ExitCode::FAILURE;
        }
    };

    match site::export_static_site(&db, &out) {
        Ok(stats) => {
            eprintln!(
                "exported {} HTML pages and {} covers → {}",
                stats.html_pages,
                stats.cover_files,
                stats.out_dir.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("export failed: {error}");
            ExitCode::FAILURE
        }
    }
}
