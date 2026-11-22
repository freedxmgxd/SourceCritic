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
    UNIQUE (outlet_id, game_id)
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
        ensure_column(&self.conn, "games", "platforms", "TEXT")?;
        ensure_column(&self.conn, "games", "synopsis", "TEXT")?;
        ensure_column(&self.conn, "games", "platform_years", "TEXT")?;
        ensure_column(&self.conn, "reviews", "excerpt", "TEXT")?;
        ensure_column(&self.conn, "reviews", "review_url", "TEXT")?;
        ensure_column(&self.conn, "reviews", "platforms", "TEXT")?;
        ensure_column(&self.conn, "reviews", "reviewed_on", "TEXT")?;
        Ok(())
    }

    pub fn insert_reviews(
        &mut self,
        incoming: &[IncomingReview],
    ) -> Result<Vec<GameView>, DbError> {
        let tx = self.conn.transaction()?;
        for review in incoming {
            tx.execute(
                "INSERT INTO outlets (slug, name, review_count, mu, sigma, mu_prime, sigma_prime)
                 VALUES (?1, ?2, 0, 0, 0, 0, 0)
                 ON CONFLICT(slug) DO UPDATE SET name = excluded.name",
                params![review.outlet_slug, review.outlet_name],
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
            tx.execute(
                "INSERT INTO reviews (
                     outlet_id, game_id, raw_score, scale_max, raw_score_norm, z_score,
                     excerpt, review_url, platforms
                 )
                 VALUES (
                     (SELECT id FROM outlets WHERE slug = ?1),
                     (SELECT id FROM games WHERE slug = ?2),
                     ?3, ?4, ?5, 0, ?6, ?7, ?8
                 )
                 ON CONFLICT(outlet_id, game_id) DO UPDATE SET
                     raw_score = excluded.raw_score,
                     scale_max = excluded.scale_max,
                     raw_score_norm = excluded.raw_score_norm,
                     excerpt = COALESCE(excluded.excerpt, reviews.excerpt),
                     review_url = COALESCE(excluded.review_url, reviews.review_url),
                     platforms = COALESCE(excluded.platforms, reviews.platforms)",
                params![
                    review.outlet_slug,
                    review.game_slug,
                    review.raw_score,
                    review.scale_max,
                    review.raw_score_norm,
                    review.excerpt,
                    review.review_url,
                    review_platforms_json,
                ],
            )?;
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
}

/// `μ_global` mudou, então todo veículo e todo jogo são reescritos.
fn recompute(conn: &Connection) -> Result<(), DbError> {
    let mut stmt = conn.prepare(
        "SELECT o.slug, g.slug, r.raw_score_norm
         FROM reviews r
         JOIN outlets o ON o.id = r.outlet_id
         JOIN games g ON g.id = r.game_id
         WHERE r.raw_score IS NOT NULL
           AND o.slug NOT IN ('metacritic-users', 'opencritic-recommend')",
    )?;
    let inputs = stmt
        .query_map([], |row| {
            Ok(ReviewInput {
                outlet_key: row.get(0)?,
                game_key: row.get(1)?,
                raw_score_norm: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    let scored = score_corpus(&inputs)?;

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
                      o.review_count, o.mu, o.sigma, o.mu_prime, o.sigma_prime
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
                title: row.title,
                cover_path: row.cover_path,
                release_year: row.release_year,
                platforms: parse_platforms(row.platforms.as_deref()),
                platform_years: parse_platform_years(row.platform_years.as_deref()),
                synopsis: row.synopsis,
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
            outlet_name: row.outlet_name,
            raw_score: row.raw_score,
            scale_max: row.scale_max,
            raw_score_norm: row.raw_score_norm,
            z_score: row.z_score,
            excerpt: row.excerpt,
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
    let mut stmt = conn.prepare("SELECT raw_score_norm FROM reviews")?;
    let values = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<f64>, _>>()?;
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
                game_title: row.get(8)?,
                raw_score: row.get(9)?,
                scale_max: row.get(10)?,
                raw_score_norm: row.get(11)?,
                z_score: row.get(12)?,
                excerpt: row.get(13)?,
                review_url: row.get(14)?,
            },
        ))
    })?;

    let mut outlets: Vec<OutletView> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for row in rows {
        let (outlet_slug, name, n, mu, sigma, mu_prime, sigma_prime, review) = row?;
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
