//! HTTP. Valida o payload e delega persistência; não calcula nota.

use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

use crate::db::{Database, GameView, IncomingReview, ReviewView};

pub mod pages;

#[derive(Clone)]
pub struct AppState {
    db: Arc<Mutex<Database>>,
}

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound,
    Internal(String),
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "game not found".to_string()),
            ApiError::Internal(message) => {
                eprintln!("internal error: {message}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

pub fn app(db: Database) -> Router {
    let state = AppState {
        db: Arc::new(Mutex::new(db)),
    };
    Router::new()
        .route("/", get(pages::home))
        .route("/game/{slug}", get(pages::game))
        .route("/outlets", get(pages::outlets))
        .route("/outlet/{slug}", get(pages::outlet))
        .route("/score", get(pages::method))
        .route("/about", get(pages::about))
        .route("/reviews", post(create_reviews))
        .route("/games", get(list_games))
        .route("/games/{slug}", get(get_game))
        .nest_service("/covers", ServeDir::new("static/covers"))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum IngestBody {
    Batch { reviews: Vec<ReviewIn> },
    One(ReviewIn),
}

#[derive(Debug, Deserialize)]
struct ReviewIn {
    outlet_slug: String,
    outlet_name: String,
    game_slug: String,
    game_title: String,
    raw_score_norm: f64,
    #[serde(default)]
    cover_path: Option<String>,
    #[serde(default)]
    release_year: Option<i32>,
    #[serde(default)]
    platforms: Vec<String>,
    #[serde(default)]
    synopsis: Option<String>,
    #[serde(default)]
    raw_score: Option<f64>,
    #[serde(default)]
    scale_max: Option<f64>,
    #[serde(default)]
    excerpt: Option<String>,
    #[serde(default)]
    review_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct IngestResponse {
    upserted: usize,
    games: Vec<GameResponse>,
}

#[derive(Debug, Serialize)]
struct GameResponse {
    slug: String,
    title: String,
    cover_path: Option<String>,
    release_year: Option<i32>,
    platforms: Vec<String>,
    synopsis: Option<String>,
    agg_score: Option<f64>,
    simple_mean: Option<f64>,
    review_count: i64,
    mu_global: f64,
    sigma_global: f64,
    reviews: Vec<ReviewResponse>,
}

#[derive(Debug, Serialize)]
struct ReviewResponse {
    outlet_slug: String,
    outlet_name: String,
    raw_score: Option<f64>,
    scale_max: Option<f64>,
    raw_score_norm: f64,
    z_score: f64,
    excerpt: Option<String>,
    review_url: Option<String>,
    outlet_n: i64,
    mu: f64,
    sigma: f64,
    mu_prime: f64,
    sigma_prime: f64,
}

impl From<GameView> for GameResponse {
    fn from(game: GameView) -> Self {
        Self {
            slug: game.slug,
            title: game.title,
            cover_path: game.cover_path,
            release_year: game.release_year,
            platforms: game.platforms,
            synopsis: game.synopsis,
            agg_score: game.agg_score,
            simple_mean: game.simple_mean,
            review_count: game.review_count,
            mu_global: game.mu_global,
            sigma_global: game.sigma_global,
            reviews: game.reviews.into_iter().map(ReviewResponse::from).collect(),
        }
    }
}

impl From<ReviewView> for ReviewResponse {
    fn from(review: ReviewView) -> Self {
        Self {
            outlet_slug: review.outlet_slug,
            outlet_name: review.outlet_name,
            raw_score: review.raw_score,
            scale_max: review.scale_max,
            raw_score_norm: review.raw_score_norm,
            z_score: review.z_score,
            excerpt: review.excerpt,
            review_url: review.review_url,
            outlet_n: review.outlet_n,
            mu: review.mu,
            sigma: review.sigma,
            mu_prime: review.mu_prime,
            sigma_prime: review.sigma_prime,
        }
    }
}

async fn create_reviews(
    State(state): State<AppState>,
    Json(body): Json<IngestBody>,
) -> Result<Json<IngestResponse>, ApiError> {
    let reviews = match body {
        IngestBody::Batch { reviews } => reviews,
        IngestBody::One(review) => vec![review],
    };
    if reviews.is_empty() {
        return Err(ApiError::BadRequest("empty batch".to_string()));
    }
    let incoming = reviews
        .iter()
        .map(validate)
        .collect::<Result<Vec<_>, _>>()?;

    let games = {
        let mut db = lock(&state)?;
        db.insert_reviews(&incoming)
            .map_err(|error| ApiError::Internal(error.to_string()))?
    };
    Ok(Json(IngestResponse {
        upserted: incoming.len(),
        games: games.into_iter().map(GameResponse::from).collect(),
    }))
}

async fn list_games(State(state): State<AppState>) -> Result<Json<Vec<GameResponse>>, ApiError> {
    let games = {
        let db = lock(&state)?;
        db.list_games()
            .map_err(|error| ApiError::Internal(error.to_string()))?
    };
    Ok(Json(games.into_iter().map(GameResponse::from).collect()))
}

async fn get_game(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<GameResponse>, ApiError> {
    let game = {
        let db = lock(&state)?;
        db.get_game(&slug)
            .map_err(|error| ApiError::Internal(error.to_string()))?
            .ok_or(ApiError::NotFound)?
    };
    Ok(Json(GameResponse::from(game)))
}

pub(super) fn lock(state: &AppState) -> Result<std::sync::MutexGuard<'_, Database>, ApiError> {
    state
        .db
        .lock()
        .map_err(|_| ApiError::Internal("lock poisoned".to_string()))
}

fn validate(review: &ReviewIn) -> Result<IncomingReview, ApiError> {
    let outlet_slug = review.outlet_slug.trim();
    let outlet_name = review.outlet_name.trim();
    let game_slug = review.game_slug.trim();
    let game_title = review.game_title.trim();
    if outlet_slug.is_empty() || game_slug.is_empty() {
        return Err(ApiError::BadRequest("empty slug".to_string()));
    }
    if outlet_name.is_empty() || game_title.is_empty() {
        return Err(ApiError::BadRequest("empty name".to_string()));
    }
    if !review.raw_score_norm.is_finite() || !(0.0..=100.0).contains(&review.raw_score_norm) {
        return Err(ApiError::BadRequest(
            "raw_score_norm must be between 0 and 100".to_string(),
        ));
    }
    if let Some(scale_max) = review.scale_max
        && (!scale_max.is_finite() || scale_max <= 0.0)
    {
        return Err(ApiError::BadRequest(
            "scale_max must be positive".to_string(),
        ));
    }
    if let Some(raw_score) = review.raw_score
        && !raw_score.is_finite()
    {
        return Err(ApiError::BadRequest("raw_score is not finite".to_string()));
    }
    let cover_path = review
        .cover_path
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(path) = &cover_path
        && (!path.starts_with("/covers/") || path.contains(".."))
    {
        return Err(ApiError::BadRequest(
            "cover_path must be under /covers/".to_string(),
        ));
    }
    let excerpt = review
        .excerpt
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let review_url = review
        .review_url
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(url) = &review_url
        && !(url.starts_with("https://") || url.starts_with("http://"))
    {
        return Err(ApiError::BadRequest(
            "review_url must be an http(s) link".to_string(),
        ));
    }
    let release_year = match review.release_year {
        Some(year) if (1970..=2100).contains(&year) => Some(year),
        Some(_) => {
            return Err(ApiError::BadRequest(
                "release_year must be between 1970 and 2100".to_string(),
            ));
        }
        None => None,
    };
    let platforms = review
        .platforms
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let synopsis = review
        .synopsis
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    Ok(IncomingReview {
        outlet_slug: outlet_slug.to_string(),
        outlet_name: outlet_name.to_string(),
        game_slug: game_slug.to_string(),
        game_title: game_title.to_string(),
        cover_path,
        release_year,
        platforms,
        synopsis,
        raw_score: review.raw_score,
        scale_max: review.scale_max,
        raw_score_norm: review.raw_score_norm,
        excerpt,
        review_url,
    })
}
