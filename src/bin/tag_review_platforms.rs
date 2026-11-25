//! Save the platform a review actually names. Reviews that name none stay empty
//! and the page treats them as the game's own platforms.

use rusqlite::{Connection, params};
use sourcecritic::api::pages::mentioned_platforms;

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let db_path = format!("{root}/sourcecritic.db");
    let conn = Connection::open(&db_path).expect("open db");
    let mut read = conn
        .prepare("SELECT id, excerpt, review_url, platforms FROM reviews")
        .expect("select");
    let rows = read
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .expect("rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect");
    drop(read);
    let mut tagged = 0usize;
    for (id, excerpt, url, existing) in rows {
        if existing.as_deref().is_some_and(|value| !value.is_empty() && value != "[]") {
            continue;
        }
        let mut text = excerpt.unwrap_or_default();
        if let Some(url) = url {
            text.push(' ');
            text.push_str(&url);
        }
        let keys = mentioned_platforms(&text);
        if keys.is_empty() {
            continue;
        }
        let json = serde_json::to_string(&keys).expect("json");
        conn.execute(
            "UPDATE reviews SET platforms = ?1 WHERE id = ?2",
            params![json, id],
        )
        .expect("update");
        tagged += 1;
    }
    println!("tagged {tagged} reviews in {db_path}");
}
