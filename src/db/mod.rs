//! Persistência. Grava reviews e aplica o resultado do motor; não calcula estatística.

use std::collections::HashMap;

use rusqlite::{Connection, params};

use crate::engine::{ReviewInput, ScoringError, mean_and_population_std, score_corpus};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS outlets (
    id INTEGER PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    review_count INTEGER NOT NULL,
    mu REAL NOT NULL,
    sigma REAL NOT NULL,
    mu_prime REAL NOT NULL,
    sigma_prime REAL NOT NULL
);

CREATE TABLE IF NOT EXISTS games (
    id INTEGER PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    cover_path TEXT,
    release_year INTEGER,
    platforms TEXT,
    agg_score REAL,
    simple_mean REAL,
    review_count INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS reviews (
    id INTEGER PRIMARY KEY,
    outlet_id INTEGER NOT NULL REFERENCES outlets(id),
    game_id INTEGER NOT NULL REFERENCES games(id),
    raw_score REAL,
    scale_max REAL,
    raw_score_norm REAL NOT NULL,
    z_score REAL NOT NULL,
    excerpt TEXT,
    review_url TEXT,
    reviewed_on TEXT,
    platforms TEXT,
    reviewer TEXT
);
"#;

#[derive(Debug)]
pub enum DbError {
    Sqlite(rusqlite::Error),
    Scoring(ScoringError),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Sqlite(error) => write!(f, "{error}"),
            DbError::Scoring(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<rusqlite::Error> for DbError {
    fn from(error: rusqlite::Error) -> Self {
        DbError::Sqlite(error)
    }
}

impl From<ScoringError> for DbError {
    fn from(error: ScoringError) -> Self {
        DbError::Scoring(error)
    }
}

#[derive(Debug, Clone)]
pub struct IncomingReview {
    pub outlet_slug: String,
    pub outlet_name: String,
    pub game_slug: String,
    pub game_title: String,
    pub cover_path: Option<String>,
    pub release_year: Option<i32>,
    pub platforms: Vec<String>,
    pub synopsis: Option<String>,
    pub raw_score: Option<f64>,
    pub scale_max: Option<f64>,
    pub raw_score_norm: f64,
    pub excerpt: Option<String>,
    pub review_url: Option<String>,
    /// Platforms this specific review covers (MC port SKUs), not the game catalog list.
    pub review_platforms: Vec<String>,
    pub reviewed_on: Option<String>,
    pub reviewer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReviewView {
    pub outlet_slug: String,
    pub outlet_name: String,
    pub raw_score: Option<f64>,
    pub scale_max: Option<f64>,
    pub raw_score_norm: f64,
    pub z_score: f64,
    pub excerpt: Option<String>,
    pub review_url: Option<String>,
    pub platforms: Vec<String>,
    pub outlet_n: i64,
    pub mu: f64,
    pub sigma: f64,
    pub mu_prime: f64,
    pub sigma_prime: f64,
}

#[derive(Debug, Clone)]
pub struct GameView {
    pub slug: String,
    pub title: String,
    pub cover_path: Option<String>,
    pub release_year: Option<i32>,
    /// ISO date (YYYY-MM-DD) of this edition's first release, when it is known.
    pub released_on: Option<String>,
    pub platforms: Vec<String>,
    /// Release year on each platform, when it is known separately from the original year.
    pub platform_years: Vec<(String, i32)>,
    pub synopsis: Option<String>,
    pub agg_score: Option<f64>,
    pub simple_mean: Option<f64>,
    pub review_count: i64,
    pub mu_global: f64,
    pub sigma_global: f64,
    pub reviews: Vec<ReviewView>,
}

#[derive(Debug, Clone)]
pub struct OutletReviewView {
    pub game_slug: String,
    pub game_title: String,
    pub raw_score: Option<f64>,
    pub scale_max: Option<f64>,
    pub raw_score_norm: f64,
    pub z_score: f64,
    pub excerpt: Option<String>,
    pub review_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OutletView {
    pub slug: String,
    pub name: String,
    pub n: i64,
    pub mu: f64,
    pub sigma: f64,
    pub mu_prime: f64,
    pub sigma_prime: f64,
    pub mu_global: f64,
    pub sigma_global: f64,
    pub reviews: Vec<OutletReviewView>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &str) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        // Export and the live server share this file. Wait instead of 500ing
        // on SQLITE_BUSY, and use WAL so reads proceed during score writes.
        conn.busy_timeout(std::time::Duration::from_secs(30))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let db = Self { conn };
        db.prepare()?;
        // Rebuild scores so hyperparameter / mapping changes apply.
        recompute(&db.conn)?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.busy_timeout(std::time::Duration::from_secs(30))?;
        let db = Self { conn };
        db.prepare()?;
        Ok(db)
    }

    fn prepare(&self) -> Result<(), DbError> {
        self.conn.pragma_update(None, "foreign_keys", "ON")?;
        self.conn.execute_batch(SCHEMA)?;
        ensure_column(&self.conn, "games", "cover_path", "TEXT")?;
        ensure_column(&self.conn, "games", "release_year", "INTEGER")?;
        ensure_column(&self.conn, "games", "released_on", "TEXT")?;
        ensure_column(&self.conn, "games", "platforms", "TEXT")?;
        ensure_column(&self.conn, "games", "synopsis", "TEXT")?;
        ensure_column(&self.conn, "games", "platform_years", "TEXT")?;
        ensure_column(&self.conn, "reviews", "excerpt", "TEXT")?;
        ensure_column(&self.conn, "reviews", "review_url", "TEXT")?;
        ensure_column(&self.conn, "reviews", "platforms", "TEXT")?;
        ensure_column(&self.conn, "reviews", "reviewed_on", "TEXT")?;
        ensure_column(&self.conn, "reviews", "reviewer", "TEXT")?;
        drop_review_outlet_game_unique(&self.conn)?;
        dedupe_stored_reviews(&self.conn)?;
        Ok(())
    }

    pub fn insert_reviews(
        &mut self,
        incoming: &[IncomingReview],
    ) -> Result<Vec<GameView>, DbError> {
        let tx = self.conn.transaction()?;
        for review in incoming {
            let outlet_slug = resolve_outlet_slug(&tx, &review.outlet_slug)?;
            tx.execute(
                "INSERT INTO outlets (slug, name, review_count, mu, sigma, mu_prime, sigma_prime)
                 VALUES (?1, ?2, 0, 0, 0, 0, 0)
                 ON CONFLICT(slug) DO UPDATE SET name = excluded.name",
                params![outlet_slug, review.outlet_name],
            )?;
            let platforms_json = if review.platforms.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&review.platforms).unwrap_or_default())
            };
            tx.execute(
                "INSERT INTO games (
                     slug, title, cover_path, release_year, platforms, synopsis,
                     agg_score, simple_mean, review_count
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, 0)
                 ON CONFLICT(slug) DO UPDATE SET
                     title = excluded.title,
                     cover_path = COALESCE(excluded.cover_path, games.cover_path),
                     release_year = COALESCE(excluded.release_year, games.release_year),
                     platforms = COALESCE(excluded.platforms, games.platforms),
                     synopsis = COALESCE(excluded.synopsis, games.synopsis)",
                params![
                    review.game_slug,
                    review.game_title,
                    review.cover_path,
                    review.release_year,
                    platforms_json,
                    review.synopsis,
                ],
            )?;
            let review_platforms_json = if review.review_platforms.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&review.review_platforms).unwrap_or_default())
            };
            upsert_review_row(&tx, review, &outlet_slug, review_platforms_json.as_deref())?;
        }
        recompute(&tx)?;
        let mut slugs: Vec<String> = Vec::new();
        for review in incoming {
            if !slugs.contains(&review.game_slug) {
                slugs.push(review.game_slug.clone());
            }
        }
        let mut games = Vec::with_capacity(slugs.len());
        for slug in slugs {
            if let Some(game) = load_game(&tx, &slug)? {
                games.push(game);
            }
        }
        tx.commit()?;
        Ok(games)
    }

    pub fn list_games(&self) -> Result<Vec<GameView>, DbError> {
        load_games(&self.conn, None)
    }

    pub fn get_game(&self, slug: &str) -> Result<Option<GameView>, DbError> {
        load_game(&self.conn, slug)
    }

    pub fn list_outlets(&self) -> Result<Vec<OutletView>, DbError> {
        load_outlets(&self.conn, None)
    }

    pub fn get_outlet(&self, slug: &str) -> Result<Option<OutletView>, DbError> {
        let mut outlets = load_outlets(&self.conn, Some(slug))?;
        Ok(outlets.pop())
    }

    /// Games that still list one outlet and one normalized URL more than once.
    pub fn duplicate_review_pairs(&self) -> Result<Vec<String>, DbError> {
        duplicate_review_pairs(&self.conn)
    }
}

/// A grade counts only when the original article is an openable http(s) page.
/// Kotaku, user scores, and recommend meters never count. Hub and parked URLs do not.
const SCORED_REVIEW_WHERE: &str = "r.raw_score IS NOT NULL
           AND o.slug NOT IN ('metacritic-users', 'opencritic-recommend', 'kotaku')";

pub(crate) fn is_hub_url(url: &str) -> bool {
    let Some(after_scheme) = url.split("://").nth(1) else {
        return true;
    };
    let cut = after_scheme.find(['?', '#']).unwrap_or(after_scheme.len());
    let path = &after_scheme[..cut];
    let query = if cut < after_scheme.len() { &after_scheme[cut + 1..] } else { "" };
    let mut parts: Vec<String> = path
        .split('/')
        .filter(|part| !part.is_empty())
        .skip(1)
        .map(|part| part.to_ascii_lowercase())
        .collect();
    const LOCALES: &[&str] = &["en", "uk", "au", "us", "br", "fr", "de", "es", "jp"];
    while parts.first().is_some_and(|part| LOCALES.contains(&part.as_str())) {
        parts.remove(0);
    }
    if parts.is_empty() {
        return query.is_empty();
    }
    const SECTIONS: &[&str] = &[
        "xbox",
        "playstation",
        "pc",
        "nintendo",
        "switch",
        "ps5",
        "ps4",
        "ps3",
        "reviews",
        "review",
        "gaming",
        "games",
        "video-games",
        "news",
        "404",
        "review-guidelines",
        "casino-sites",
        "actualite.html",
    ];
    let last = parts[parts.len() - 1].as_str();
    if last.contains("suspendedpage")
        || last == "404"
        || last == "review-guidelines"
        || last == "casino-sites"
        || last == "actualite.html"
    {
        return true;
    }
    parts.iter().all(|part| SECTIONS.contains(&part.as_str()))
}

/// Same check the page uses before it prints "Original inaccessible".
pub(crate) fn openable_original(url: Option<&str>) -> bool {
    url.is_some_and(|url| {
        (url.starts_with("https://") || url.starts_with("http://"))
            && !url.contains("savethechildren.org")
            && !is_hub_url(url)
    })
}

/// `μ_global` mudou, então todo veículo e todo jogo são reescritos.
fn recompute(conn: &Connection) -> Result<(), DbError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT o.slug, g.slug, r.raw_score_norm, r.review_url
         FROM reviews r
         JOIN outlets o ON o.id = r.outlet_id
         JOIN games g ON g.id = r.game_id
         WHERE {SCORED_REVIEW_WHERE}"
    ))?;
    let inputs = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|(_, _, _, url)| openable_original(url.as_deref()))
        .map(|(outlet_key, game_key, raw_score_norm, _)| ReviewInput {
            outlet_key,
            game_key,
            raw_score_norm,
        })
        .collect::<Vec<_>>();
    drop(stmt);

    let scored = score_corpus(&inputs)?;

    // Savepoint so a killed process rolls the old totals back, and so this
    // still works when the caller (insert) already opened a transaction.
    conn.execute("SAVEPOINT sourcecritic_recompute", [])?;
    let written = apply_scores(conn, &scored);
    if written.is_err() {
        let _ = conn.execute("ROLLBACK TO sourcecritic_recompute", []);
    }
    conn.execute("RELEASE sourcecritic_recompute", [])?;
    written
}

fn apply_scores(conn: &Connection, scored: &crate::engine::ScoringResult) -> Result<(), DbError> {
    // Scores above were computed only from numeric critic reviews. Clear the
    // rest so a blocked or unscored row cannot keep an old total.
    conn.execute(
        "UPDATE outlets SET review_count = 0, mu = 0, sigma = 0, mu_prime = 0, sigma_prime = 0",
        [],
    )?;
    conn.execute(
        "UPDATE games SET agg_score = NULL, simple_mean = NULL, review_count = 0",
        [],
    )?;

    for outlet in &scored.outlets {
        conn.execute(
            "UPDATE outlets
             SET review_count = ?1, mu = ?2, sigma = ?3, mu_prime = ?4, sigma_prime = ?5
             WHERE slug = ?6",
            params![
                outlet.n as i64,
                outlet.mu,
                outlet.sigma,
                outlet.mu_prime,
                outlet.sigma_prime,
                outlet.outlet_key,
            ],
        )?;
    }
    for review in &scored.reviews {
        conn.execute(
            "UPDATE reviews
             SET z_score = ?1
             WHERE outlet_id = (SELECT id FROM outlets WHERE slug = ?2)
               AND game_id = (SELECT id FROM games WHERE slug = ?3)",
            params![review.z_score, review.outlet_key, review.game_key],
        )?;
    }
    for game in &scored.games {
        conn.execute(
            "UPDATE games
             SET agg_score = ?1, simple_mean = ?2, review_count = ?3
             WHERE slug = ?4",
            params![
                game.agg_score,
                game.simple_mean,
                game.review_count as i64,
                game.game_key,
            ],
        )?;
    }
    Ok(())
}

fn load_game(conn: &Connection, slug: &str) -> Result<Option<GameView>, DbError> {
    let games = load_games(conn, Some(slug))?;
    Ok(games.into_iter().next())
}

fn load_games(conn: &Connection, slug: Option<&str>) -> Result<Vec<GameView>, DbError> {
    let (mu_global, sigma_global) = global_moments(conn)?;
    let sql = "SELECT g.slug, g.title, g.cover_path, g.release_year, g.platforms,
                      g.synopsis, g.platform_years,
                      g.agg_score, g.simple_mean, g.review_count,
                      o.slug, o.name, r.raw_score, r.scale_max, r.raw_score_norm, r.z_score,
                      r.excerpt, r.review_url, r.platforms,
                      o.review_count, o.mu, o.sigma, o.mu_prime, o.sigma_prime,
                      g.released_on
               FROM games g
               JOIN reviews r ON r.game_id = g.id
               JOIN outlets o ON o.id = r.outlet_id
               WHERE (?1 IS NULL OR g.slug = ?1)
               ORDER BY g.agg_score DESC, g.slug ASC, o.slug ASC";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![slug], |row| {
        Ok(FlatRow {
            game_slug: row.get(0)?,
            title: row.get(1)?,
            cover_path: row.get(2)?,
            release_year: row.get(3)?,
            platforms: row.get(4)?,
            synopsis: row.get(5)?,
            platform_years: row.get(6)?,
            agg_score: row.get(7)?,
            simple_mean: row.get(8)?,
            review_count: row.get(9)?,
            outlet_slug: row.get(10)?,
            outlet_name: row.get(11)?,
            raw_score: row.get(12)?,
            scale_max: row.get(13)?,
            raw_score_norm: row.get(14)?,
            z_score: row.get(15)?,
            excerpt: row.get(16)?,
            review_url: row.get(17)?,
            review_platforms: row.get(18)?,
            outlet_n: row.get(19)?,
            mu: row.get(20)?,
            sigma: row.get(21)?,
            mu_prime: row.get(22)?,
            sigma_prime: row.get(23)?,
            released_on: row.get(24)?,
        })
    })?;

    let mut games: Vec<GameView> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for row in rows {
        let row = row?;
        let position = if let Some(position) = index.get(&row.game_slug) {
            *position
        } else {
            let position = games.len();
            index.insert(row.game_slug.clone(), position);
            games.push(GameView {
                slug: row.game_slug.clone(),
                title: decode_entities(&row.title),
                cover_path: row.cover_path,
                release_year: row.release_year,
                released_on: row.released_on,
                platforms: parse_platforms(row.platforms.as_deref()),
                platform_years: parse_platform_years(row.platform_years.as_deref()),
                synopsis: row.synopsis.as_deref().map(decode_entities),
                agg_score: row.agg_score,
                simple_mean: row.simple_mean,
                review_count: row.review_count,
                mu_global,
                sigma_global,
                reviews: Vec::new(),
            });
            position
        };
        games[position].reviews.push(ReviewView {
            outlet_slug: row.outlet_slug,
            outlet_name: decode_entities(&row.outlet_name),
            raw_score: row.raw_score,
            scale_max: row.scale_max,
            raw_score_norm: row.raw_score_norm,
            z_score: row.z_score,
            excerpt: row.excerpt.as_deref().map(decode_entities),
            review_url: row.review_url,
            platforms: parse_platforms(row.review_platforms.as_deref()),
            outlet_n: row.outlet_n,
            mu: row.mu,
            sigma: row.sigma,
            mu_prime: row.mu_prime,
            sigma_prime: row.sigma_prime,
        });
    }
    Ok(games)
}

/// Lowercase host, no query, no fragment, no trailing slash.
///
/// A leading `www.` is removed and the port is not part of the host, so
/// `https://www.jeuxvideo.com:443/a/?q=1#x` and `https://jeuxvideo.com/a` match.
pub fn normalize_review_url(raw: &str) -> String {
    let raw = raw.trim();
    let Some((scheme, rest)) = raw.split_once("://") else {
        return String::new();
    };
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return String::new();
    }
    let rest = rest.split(['?', '#']).next().unwrap_or(rest);
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (rest, String::new()),
    };
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let host = if let Some(ipv6) = authority.strip_prefix('[') {
        ipv6.split(']').next().unwrap_or(ipv6)
    } else {
        authority.split(':').next().unwrap_or(authority)
    };
    let lower = host.to_ascii_lowercase();
    let host = lower.strip_prefix("www.").unwrap_or(&lower);
    if host.is_empty() {
        return String::new();
    }
    let path = path.trim_end_matches('/');
    format!("{scheme}://{host}{path}")
}

fn score_token(score: f64) -> String {
    format!("{score:.2}")
}

fn reviewer_token(reviewer: Option<&str>) -> String {
    reviewer
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Outlet plus normalized URL. An empty URL uses outlet, score, and reviewer.
pub fn review_dedupe_key(
    outlet_slug: &str,
    url: Option<&str>,
    score: f64,
    reviewer: Option<&str>,
) -> String {
    let norm = url.map(normalize_review_url).unwrap_or_default();
    if !norm.is_empty() {
        format!("url\u{1}{outlet_slug}\u{1}{norm}")
    } else {
        format!(
            "bare\u{1}{outlet_slug}\u{1}{}\u{1}{}",
            score_token(score),
            reviewer_token(reviewer)
        )
    }
}

fn hyphen_flat(slug: &str) -> String {
    slug.chars().filter(|ch| *ch != '-').collect()
}

fn resolve_outlet_slug(conn: &Connection, slug: &str) -> Result<String, DbError> {
    let mut stmt = conn.prepare("SELECT slug FROM outlets")?;
    let existing = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if existing.iter().any(|item| item == slug) {
        return Ok(slug.to_string());
    }
    let flat = hyphen_flat(slug);
    let mut hits: Vec<String> = existing
        .into_iter()
        .filter(|item| hyphen_flat(item) == flat)
        .collect();
    if hits.len() == 1 {
        return Ok(hits.remove(0));
    }
    Ok(slug.to_string())
}

fn richness(url: Option<&str>, score: Option<f64>, date: Option<&str>) -> (u8, u8, u8, u8) {
    let has_url = url.is_some_and(|value| !normalize_review_url(value).is_empty()) as u8;
    let has_score = score.is_some() as u8;
    let has_date = date.is_some_and(|value| !value.trim().is_empty()) as u8;
    (has_url + has_score + has_date, has_url, has_score, has_date)
}

fn pair_ok(raw: Option<f64>, scale: Option<f64>, norm: f64) -> bool {
    match (raw, scale) {
        (Some(raw), Some(scale)) if scale > 0.0 => (raw / scale * 100.0 - norm).abs() < 1.5,
        _ => false,
    }
}

fn text_blank(value: &Option<String>) -> bool {
    value.as_deref().map(str::trim).unwrap_or("").is_empty()
}

fn coalesce_text(primary: Option<String>, fallback: Option<String>) -> Option<String> {
    if text_blank(&primary) {
        fallback.filter(|value| !value.trim().is_empty())
    } else {
        primary
    }
}

#[derive(Clone)]
struct StoredReview {
    id: i64,
    url: Option<String>,
    raw_score: Option<f64>,
    scale_max: Option<f64>,
    raw_score_norm: f64,
    excerpt: Option<String>,
    platforms: Option<String>,
    reviewed_on: Option<String>,
    reviewer: Option<String>,
}

fn upsert_review_row(
    conn: &Connection,
    review: &IncomingReview,
    outlet_slug: &str,
    platforms_json: Option<&str>,
) -> Result<(), DbError> {
    let key = review_dedupe_key(
        outlet_slug,
        review.review_url.as_deref(),
        review.raw_score_norm,
        review.reviewer.as_deref(),
    );
    let mut stmt = conn.prepare(
        "SELECT r.id, r.raw_score, r.scale_max, r.raw_score_norm, r.excerpt, r.review_url,
                r.platforms, r.reviewed_on, r.reviewer
         FROM reviews r
         JOIN outlets o ON o.id = r.outlet_id
         JOIN games g ON g.id = r.game_id
         WHERE o.slug = ?1 AND g.slug = ?2
         ORDER BY r.id",
    )?;
    let rows = stmt.query_map(params![outlet_slug, review.game_slug], |row| {
        Ok(StoredReview {
            id: row.get(0)?,
            raw_score: row.get(1)?,
            scale_max: row.get(2)?,
            raw_score_norm: row.get(3)?,
            excerpt: row.get(4)?,
            url: row.get(5)?,
            platforms: row.get(6)?,
            reviewed_on: row.get(7)?,
            reviewer: row.get(8)?,
        })
    })?;
    let mut found = None;
    for row in rows {
        let row = row?;
        let existing_key = review_dedupe_key(
            outlet_slug,
            row.url.as_deref(),
            row.raw_score_norm,
            row.reviewer.as_deref(),
        );
        if existing_key == key {
            found = Some(row);
            break;
        }
    }
    drop(stmt);

    let Some(existing) = found else {
        conn.execute(
            "INSERT INTO reviews (
                 outlet_id, game_id, raw_score, scale_max, raw_score_norm, z_score,
                 excerpt, review_url, platforms, reviewed_on, reviewer
             )
             VALUES (
                 (SELECT id FROM outlets WHERE slug = ?1),
                 (SELECT id FROM games WHERE slug = ?2),
                 ?3, ?4, ?5, 0, ?6, ?7, ?8, ?9, ?10
             )",
            params![
                outlet_slug,
                review.game_slug,
                review.raw_score,
                review.scale_max,
                review.raw_score_norm,
                review.excerpt,
                review.review_url,
                platforms_json,
                review.reviewed_on,
                review.reviewer,
            ],
        )?;
        return Ok(());
    };

    let incoming_rank = richness(
        review.review_url.as_deref(),
        review.raw_score,
        review.reviewed_on.as_deref(),
    );
    let stored_rank = richness(
        existing.url.as_deref(),
        existing.raw_score,
        existing.reviewed_on.as_deref(),
    );
    let incoming_wins = incoming_rank > stored_rank
        || (incoming_rank == stored_rank
            && pair_ok(review.raw_score, review.scale_max, review.raw_score_norm)
            && !pair_ok(existing.raw_score, existing.scale_max, existing.raw_score_norm));
    let (raw_score, scale_max, raw_score_norm, excerpt, review_url, platforms, reviewed_on, reviewer) =
        if incoming_wins {
            (
                review.raw_score.or(existing.raw_score),
                review.scale_max.or(existing.scale_max),
                if review.raw_score.is_some() {
                    review.raw_score_norm
                } else {
                    existing.raw_score_norm
                },
                coalesce_text(review.excerpt.clone(), existing.excerpt.clone()),
                coalesce_text(review.review_url.clone(), existing.url.clone()),
                coalesce_text(platforms_json.map(str::to_string), existing.platforms.clone()),
                coalesce_text(review.reviewed_on.clone(), existing.reviewed_on.clone()),
                coalesce_text(review.reviewer.clone(), existing.reviewer.clone()),
            )
        } else {
            (
                existing.raw_score.or(review.raw_score),
                existing.scale_max.or(review.scale_max),
                if existing.raw_score.is_some() {
                    existing.raw_score_norm
                } else {
                    review.raw_score_norm
                },
                coalesce_text(existing.excerpt.clone(), review.excerpt.clone()),
                coalesce_text(existing.url.clone(), review.review_url.clone()),
                coalesce_text(existing.platforms.clone(), platforms_json.map(str::to_string)),
                coalesce_text(existing.reviewed_on.clone(), review.reviewed_on.clone()),
                coalesce_text(existing.reviewer.clone(), review.reviewer.clone()),
            )
        };
    conn.execute(
        "UPDATE reviews
         SET raw_score = ?1, scale_max = ?2, raw_score_norm = ?3, excerpt = ?4,
             review_url = ?5, platforms = ?6, reviewed_on = ?7, reviewer = ?8
         WHERE id = ?9",
        params![
            raw_score,
            scale_max,
            raw_score_norm,
            excerpt,
            review_url,
            platforms,
            reviewed_on,
            reviewer,
            existing.id,
        ],
    )?;
    Ok(())
}

fn drop_review_outlet_game_unique(conn: &Connection) -> Result<(), DbError> {
    let sql: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'reviews'",
        [],
        |row| row.get(0),
    )?;
    if !sql.to_ascii_lowercase().contains("unique") {
        conn.execute(
            "CREATE INDEX IF NOT EXISTS reviews_outlet_game ON reviews(outlet_id, game_id)",
            [],
        )?;
        return Ok(());
    }
    conn.pragma_update(None, "foreign_keys", "OFF")?;
    conn.execute_batch(
        "DROP TABLE IF EXISTS reviews_new;
         CREATE TABLE reviews_new (
            id INTEGER PRIMARY KEY,
            outlet_id INTEGER NOT NULL REFERENCES outlets(id),
            game_id INTEGER NOT NULL REFERENCES games(id),
            raw_score REAL,
            scale_max REAL,
            raw_score_norm REAL NOT NULL,
            z_score REAL NOT NULL,
            excerpt TEXT,
            review_url TEXT,
            reviewed_on TEXT,
            platforms TEXT,
            reviewer TEXT
         );
         INSERT INTO reviews_new (
            id, outlet_id, game_id, raw_score, scale_max, raw_score_norm, z_score,
            excerpt, review_url, reviewed_on, platforms, reviewer
         )
         SELECT id, outlet_id, game_id, raw_score, scale_max, raw_score_norm, z_score,
                excerpt, review_url, reviewed_on, platforms, reviewer
         FROM reviews;
         DROP TABLE reviews;
         ALTER TABLE reviews_new RENAME TO reviews;
         CREATE INDEX IF NOT EXISTS reviews_outlet_game ON reviews(outlet_id, game_id);",
    )?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

fn dedupe_stored_reviews(conn: &Connection) -> Result<(), DbError> {
    let mut stmt = conn.prepare(
        "SELECT r.id, g.slug, o.slug, r.raw_score, r.scale_max, r.raw_score_norm,
                r.excerpt, r.review_url, r.platforms, r.reviewed_on, r.reviewer
         FROM reviews r
         JOIN games g ON g.id = r.game_id
         JOIN outlets o ON o.id = r.outlet_id
         ORDER BY g.slug, r.id",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                StoredReview {
                    id: row.get(0)?,
                    raw_score: row.get(3)?,
                    scale_max: row.get(4)?,
                    raw_score_norm: row.get(5)?,
                    excerpt: row.get(6)?,
                    url: row.get(7)?,
                    platforms: row.get(8)?,
                    reviewed_on: row.get(9)?,
                    reviewer: row.get(10)?,
                },
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    let mut kept: HashMap<(String, String), StoredReview> = HashMap::new();
    let mut delete_ids = Vec::new();
    for (id, game, outlet, row) in rows {
        let _ = id;
        let key = review_dedupe_key(
            &outlet,
            row.url.as_deref(),
            row.raw_score_norm,
            row.reviewer.as_deref(),
        );
        let slot = (game, key);
        if let Some(previous) = kept.get(&slot) {
            let previous_rank = richness(
                previous.url.as_deref(),
                previous.raw_score,
                previous.reviewed_on.as_deref(),
            );
            let next_rank = richness(row.url.as_deref(), row.raw_score, row.reviewed_on.as_deref());
            let (mut base, other, drop_id) = if next_rank > previous_rank
                || (next_rank == previous_rank
                    && pair_ok(row.raw_score, row.scale_max, row.raw_score_norm)
                    && !pair_ok(previous.raw_score, previous.scale_max, previous.raw_score_norm))
            {
                (row, previous.clone(), previous.id)
            } else {
                (previous.clone(), row.clone(), row.id)
            };
            if base.raw_score.is_none() {
                base.raw_score = other.raw_score;
                base.raw_score_norm = other.raw_score_norm;
            }
            if base.scale_max.is_none() {
                base.scale_max = other.scale_max;
            }
            base.excerpt = coalesce_text(base.excerpt.clone(), other.excerpt.clone());
            base.url = coalesce_text(base.url.clone(), other.url.clone());
            base.platforms = coalesce_text(base.platforms.clone(), other.platforms.clone());
            base.reviewed_on = coalesce_text(base.reviewed_on.clone(), other.reviewed_on.clone());
            base.reviewer = coalesce_text(base.reviewer.clone(), other.reviewer.clone());
            delete_ids.push(drop_id);
            kept.insert(slot, base);
        } else {
            kept.insert(slot, row);
        }
    }
    if delete_ids.is_empty() {
        return Ok(());
    }
    let tx = conn.unchecked_transaction()?;
    for row in kept.values() {
        if delete_ids.contains(&row.id) {
            continue;
        }
        tx.execute(
            "UPDATE reviews
             SET raw_score = ?1, scale_max = ?2, raw_score_norm = ?3, excerpt = ?4,
                 review_url = ?5, platforms = ?6, reviewed_on = ?7, reviewer = ?8
             WHERE id = ?9",
            params![
                row.raw_score,
                row.scale_max,
                row.raw_score_norm,
                row.excerpt,
                row.url,
                row.platforms,
                row.reviewed_on,
                row.reviewer,
                row.id,
            ],
        )?;
    }
    for id in &delete_ids {
        tx.execute("DELETE FROM reviews WHERE id = ?1", params![id])?;
    }
    tx.commit()?;
    Ok(())
}

fn duplicate_review_pairs(conn: &Connection) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT g.slug, o.slug, r.review_url
         FROM reviews r
         JOIN games g ON g.id = r.game_id
         JOIN outlets o ON o.id = r.outlet_id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;
    let mut counts: HashMap<(String, String, String), usize> = HashMap::new();
    for row in rows {
        let (game, outlet, url) = row?;
        let norm = url.as_deref().map(normalize_review_url).unwrap_or_default();
        if norm.is_empty() {
            continue;
        }
        *counts.entry((game, outlet, norm)).or_default() += 1;
    }
    let mut dupes: Vec<String> = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|((game, outlet, url), count)| format!("{game} lists {outlet} {count} times at {url}"))
        .collect();
    dupes.sort();
    Ok(dupes)
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    decl: &str,
) -> Result<(), DbError> {
    let mut has = false;
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for name in rows {
        if name? == column {
            has = true;
            break;
        }
    }
    if !has {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"),
            [],
        )?;
    }
    Ok(())
}

fn global_moments(conn: &Connection) -> Result<(f64, f64), DbError> {
    // Same population the scorer uses. An unscored row must not move the baseline.
    let mut stmt = conn.prepare(&format!(
        "SELECT r.raw_score_norm, r.review_url
         FROM reviews r
         JOIN outlets o ON o.id = r.outlet_id
         WHERE {SCORED_REVIEW_WHERE}"
    ))?;
    let values = stmt
        .query_map([], |row| {
            Ok((row.get::<_, f64>(0)?, row.get::<_, Option<String>>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|(_, url)| openable_original(url.as_deref()))
        .map(|(score, _)| score)
        .collect::<Vec<_>>();
    Ok(mean_and_population_std(&values))
}

fn load_outlets(conn: &Connection, slug: Option<&str>) -> Result<Vec<OutletView>, DbError> {
    let (mu_global, sigma_global) = global_moments(conn)?;
    let sql = "SELECT o.slug, o.name, o.review_count, o.mu, o.sigma, o.mu_prime, o.sigma_prime,
                      g.slug, g.title, r.raw_score, r.scale_max, r.raw_score_norm, r.z_score,
                      r.excerpt, r.review_url
               FROM outlets o
               JOIN reviews r ON r.outlet_id = o.id
               JOIN games g ON g.id = r.game_id
               WHERE (?1 IS NULL OR o.slug = ?1)
               ORDER BY o.review_count DESC, o.name ASC, g.title ASC";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![slug], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
            row.get::<_, f64>(5)?,
            row.get::<_, f64>(6)?,
            OutletReviewView {
                game_slug: row.get(7)?,
                game_title: decode_entities(&row.get::<_, String>(8)?),
                raw_score: row.get(9)?,
                scale_max: row.get(10)?,
                raw_score_norm: row.get(11)?,
                z_score: row.get(12)?,
                excerpt: row.get::<_, Option<String>>(13)?.as_deref().map(decode_entities),
                review_url: row.get(14)?,
            },
        ))
    })?;

    let mut outlets: Vec<OutletView> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for row in rows {
        let (outlet_slug, name, n, mu, sigma, mu_prime, sigma_prime, review) = row?;
        let name = decode_entities(&name);
        let position = if let Some(position) = index.get(&outlet_slug) {
            *position
        } else {
            let position = outlets.len();
            index.insert(outlet_slug.clone(), position);
            outlets.push(OutletView {
                slug: outlet_slug,
                name,
                n,
                mu,
                sigma,
                mu_prime,
                sigma_prime,
                mu_global,
                sigma_global,
                reviews: Vec::new(),
            });
            position
        };
        outlets[position].reviews.push(review);
    }
    Ok(outlets)
}

struct FlatRow {
    game_slug: String,
    title: String,
    cover_path: Option<String>,
    release_year: Option<i32>,
    platforms: Option<String>,
    synopsis: Option<String>,
    platform_years: Option<String>,
    agg_score: Option<f64>,
    simple_mean: Option<f64>,
    review_count: i64,
    outlet_slug: String,
    outlet_name: String,
    raw_score: Option<f64>,
    scale_max: Option<f64>,
    raw_score_norm: f64,
    z_score: f64,
    excerpt: Option<String>,
    review_url: Option<String>,
    review_platforms: Option<String>,
    outlet_n: i64,
    mu: f64,
    sigma: f64,
    mu_prime: f64,
    sigma_prime: f64,
    released_on: Option<String>,
}

fn parse_platform_years(raw: Option<&str>) -> Vec<(String, i32)> {
    let Some(raw) = raw.filter(|value| !value.trim().is_empty()) else {
        return Vec::new();
    };
    let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(raw) else {
        return Vec::new();
    };
    let mut years = Vec::new();
    for (key, value) in map {
        if let Some(year) = value.as_i64() {
            years.push((key, year as i32));
        }
    }
    years.sort();
    years
}

/// OpenCritic stored `&` as the custom entity `&a;`, which HTML-escapes to `&amp;a;`.
pub(crate) fn decode_entities(value: &str) -> String {
    let mut text = value.to_string();
    for _ in 0..4 {
        let next = text
            .replace("&amp;a;", "&")
            .replace("&amp;", "&")
            .replace("&a;", "&")
            .replace("&q;", "\"")
            .replace("&s;", "'");
        if next == text {
            break;
        }
        text = next;
    }
    const PAIRS: &[(&str, &str)] = &[
        ("â€™", "\u{2019}"),
        ("â€œ", "\u{201c}"),
        ("â€\u{9d}", "\u{201d}"),
        ("â€“", "\u{2013}"),
        ("â€”", "\u{2014}"),
        ("Ã©", "é"),
        ("Ã¡", "á"),
        ("Ã­", "í"),
        ("Ã³", "ó"),
        ("Ãº", "ú"),
        ("Ã±", "ñ"),
        ("Ã£", "ã"),
        ("Ã§", "ç"),
        ("clichï¿½", "cliché"),
    ];
    for (bad, good) in PAIRS {
        if text.contains(*bad) {
            text = text.replace(bad, good);
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{normalize_review_url, review_dedupe_key};

    #[test]
    fn normalizes_the_review_url() {
        assert_eq!(
            normalize_review_url(
                "https://www.Jeuxvideo.com:443/test/829186/god-of-war.htm?utm=1#frag"
            ),
            "https://jeuxvideo.com/test/829186/god-of-war.htm"
        );
        assert_eq!(
            normalize_review_url("https://gameover.gr/reviews/1/"),
            "https://gameover.gr/reviews/1"
        );
        assert_eq!(
            normalize_review_url("http://gamer.no/artikler/x/"),
            "http://gamer.no/artikler/x"
        );
        assert_eq!(normalize_review_url(""), "");
    }

    #[test]
    fn dedupe_key_keeps_a_second_url_and_folds_an_empty_one() {
        assert_eq!(
            review_dedupe_key(
                "gameover-gr",
                Some("https://gameover.gr/reviews/1/"),
                100.0,
                None
            ),
            review_dedupe_key(
                "gameover-gr",
                Some("https://www.gameover.gr/reviews/1?draft=1"),
                90.0,
                Some("Ann")
            )
        );
        assert_ne!(
            review_dedupe_key("ign", Some("https://ign.com/ps5-review"), 90.0, None),
            review_dedupe_key("ign", Some("https://ign.com/pc-review"), 90.0, None)
        );
        assert_eq!(
            review_dedupe_key("games-tm", None, 90.0, None),
            review_dedupe_key("games-tm", Some(""), 90.0, Some("  "))
        );
        assert_ne!(
            review_dedupe_key("games-tm", None, 90.0, Some("Ann")),
            review_dedupe_key("games-tm", None, 90.0, Some("Bob"))
        );
    }
}

fn parse_platforms(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw.filter(|value| !value.trim().is_empty()) else {
        return Vec::new();
    };
    if let Ok(values) = serde_json::from_str::<Vec<String>>(raw) {
        return values
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
    }
    raw.split(',')
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}
