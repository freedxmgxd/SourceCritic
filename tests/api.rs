use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sourcecritic::api::app;
use sourcecritic::db::Database;
use tower::ServiceExt;

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn review(outlet: &str, game: &str, score: f64) -> Value {
    json!({
        "outlet_slug": outlet,
        "outlet_name": outlet,
        "game_slug": game,
        "game_title": game,
        "raw_score_norm": score
    })
}

#[tokio::test]
async fn home_page_is_html() {
    let app = app(Database::open_in_memory().unwrap());
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.starts_with("text/html"));
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(html.contains("SourceCritic"));
    assert!(html.contains("No games yet"));
    assert!(html.contains("href=\"/outlets\""));
    assert!(html.contains("href=\"/about\""));
    assert!(!html.contains("href=\"/submit\""));
    assert!(html.contains("lang=\"en\""));
    assert!(!html.contains("role=\"tab\""));
    // Ad slot stays off the page until SOURCECRITIC_AD_HTML / AD_HTML is set.
    assert!(!html.contains("class=\"site-ad\""));
}

#[tokio::test]
async fn post_single_review_and_read_game() {
    let app = app(Database::open_in_memory().unwrap());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/reviews")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "outlet_slug": "ign",
                        "outlet_name": "IGN",
                        "game_slug": "hades-2",
                        "game_title": "Hades II",
                        "raw_score": 8.0,
                        "scale_max": 10.0,
                        "raw_score_norm": 80.0
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created = body_json(response).await;
    assert_eq!(created["upserted"], json!(1));
    assert_eq!(created["games"][0]["agg_score"], json!(80.0));
    assert_eq!(created["games"][0]["simple_mean"], json!(80.0));
    assert_eq!(created["games"][0]["reviews"][0]["z_score"], json!(0.0));
    assert_eq!(created["games"][0]["reviews"][0]["sigma_prime"], json!(5.0));

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/games")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = body_json(listed).await;
    assert_eq!(listed[0]["slug"], json!("hades-2"));
    assert_eq!(listed[0]["review_count"], json!(1));

    let detail = app
        .oneshot(
            Request::builder()
                .uri("/games/hades-2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail = body_json(detail).await;
    assert_eq!(detail["title"], json!("Hades II"));
    assert_eq!(detail["reviews"][0]["mu_prime"], json!(80.0));
}

#[tokio::test]
async fn batch_is_idempotent_and_global_update_moves_other_games() {
    let app = app(Database::open_in_memory().unwrap());

    let first = json!({
        "reviews": [
            review("harsh", "shared", 70.0),
            review("harsh", "h0", 40.0),
            review("harsh", "h1", 40.0),
            review("harsh", "h2", 40.0)
        ]
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/reviews")
                .header("content-type", "application/json")
                .body(Body::from(first.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let before = body_json(response).await;
    let shared_before = before["games"]
        .as_array()
        .unwrap()
        .iter()
        .find(|game| game["slug"] == "shared")
        .unwrap()["agg_score"]
        .as_f64()
        .unwrap();

    let second = json!({
        "reviews": [
            review("lenient", "shared", 70.0),
            review("lenient", "l0", 90.0),
            review("lenient", "l1", 90.0),
            review("lenient", "l2", 90.0)
        ]
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/reviews")
                .header("content-type", "application/json")
                .body(Body::from(second.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let again = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/reviews")
                .header("content-type", "application/json")
                .body(Body::from(second.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(again.status(), StatusCode::OK);

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/games/shared")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let shared = body_json(listed).await;
    assert_eq!(shared["review_count"], json!(2));
    assert_eq!(shared["simple_mean"], json!(70.0));
    let agg = shared["agg_score"].as_f64().unwrap();
    assert!(
        (agg - shared_before).abs() > 1e-6,
        "agg should move after the global update"
    );

    let z: Vec<f64> = shared["reviews"]
        .as_array()
        .unwrap()
        .iter()
        .map(|review| review["z_score"].as_f64().unwrap())
        .collect();
    let harsh = shared["reviews"]
        .as_array()
        .unwrap()
        .iter()
        .find(|review| review["outlet_slug"] == "harsh")
        .unwrap()["z_score"]
        .as_f64()
        .unwrap();
    let lenient = shared["reviews"]
        .as_array()
        .unwrap()
        .iter()
        .find(|review| review["outlet_slug"] == "lenient")
        .unwrap()["z_score"]
        .as_f64()
        .unwrap();
    assert!(harsh > lenient);
    assert_eq!(z.len(), 2);

    let repeated = app
        .oneshot(
            Request::builder()
                .uri("/games/shared")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let repeated = body_json(repeated).await;
    assert_eq!(repeated["agg_score"], shared["agg_score"]);
    assert_eq!(repeated["review_count"], json!(2));
}

#[tokio::test]
async fn rejects_bad_score_and_missing_game() {
    let app = app(Database::open_in_memory().unwrap());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/reviews")
                .header("content-type", "application/json")
                .body(Body::from(review("ign", "hades", 140.0).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let missing = app
        .oneshot(
            Request::builder()
                .uri("/games/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn pages_are_separate_documents() {
    let app = app(Database::open_in_memory().unwrap());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/reviews")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "outlet_slug": "ign",
                        "outlet_name": "IGN",
                        "game_slug": "hades-ii",
                        "game_title": "Hades II",
                        "raw_score": 8.0,
                        "scale_max": 10.0,
                        "raw_score_norm": 80.0
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/game/hades-ii")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    let html = String::from_utf8(
        page.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains("score-hero-value"));
    assert!(html.contains(">80</span>"));
    assert!(html.contains("id=\"game-reviews\""));
    assert!(html.contains("data-sort"));

    let outlets = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/outlets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(outlets.status(), StatusCode::OK);
    let outlets = String::from_utf8(
        outlets
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(outlets.contains("href=\"/outlet/ign\""));
    assert!(outlets.contains("IGN"));

    let outlet = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/outlet/ign")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(outlet.status(), StatusCode::OK);
    let outlet = String::from_utf8(
        outlet
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(outlet.contains("μ<sub>j</sub>"));
    assert!(outlet.contains("σ<sub>j</sub>"));
    assert!(outlet.contains("Hades II"));
    assert!(outlet.contains("href=\"/game/hades-ii\""));

    let method = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/score")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let method = String::from_utf8(
        method
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(method.contains("def game_score"));
    assert!(method.contains(r"Z_{i,j} = \frac{x_{i,j} - \mu'_j}{\sigma'_j}"));
    assert!(method.contains(r"\mathrm{Score} = \frac{53.30 + 84.90}{2} = 69.10"));
    assert!(method.contains("def map_z"));
    assert!(method.contains("Deviation vs the same outlet"));
    assert!(method.contains("Convert each deviation to 0–100"));
    assert!(method.contains("Average the converted scores"));
    assert!(method.contains("katex"));
    assert!(method.contains("pulls it <em>down</em>"));
    assert!(method.contains("Why this is fair"));
    assert!(method.contains("class=\"walk\""));
    assert!(method.contains("class=\"panel\""));
    assert!(method.contains("class=\"block math\""));
    assert!(method.contains("class=\"block code\""));

    let home = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let home = String::from_utf8(
        home.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(home.contains("href=\"/game/hades-ii\""));
    assert!(home.contains("name=\"platform\""));
    assert!(home.contains("name=\"year\""));
    assert!(home.contains("name=\"min_reviews\""));
    assert!(!home.contains("IGN"));
    assert!(home.contains("href=\"/outlets\""));
    assert!(home.contains("href=\"/about\""));
    assert!(!home.contains("href=\"/submit\""));

    let about = app
        .oneshot(
            Request::builder()
                .uri("/about")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(about.status(), StatusCode::OK);
    let about = String::from_utf8(
        about
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(about.contains("FAQ, contact"));
    assert!(about.contains("id=\"contact\""));
    assert!(about.contains("id=\"donate\""));
    assert!(about.contains("coming soon"));
    assert!(!about.contains("mailto:"));
    assert!(about.contains("aria-current=\"page\""));
}
