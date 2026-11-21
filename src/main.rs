use std::env;

use sourcecritic::api::app;
use sourcecritic::db::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::var("SOURCECRITIC_DB").unwrap_or_else(|_| "sourcecritic.db".to_string());
    let db = Database::open(&path)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    eprintln!("sourcecritic at http://127.0.0.1:3000");
    axum::serve(listener, app(db)).await?;
    Ok(())
}
