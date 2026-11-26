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
        "raw_score_norm": score,
        "review_url": format!("https://example.com/{outlet}/{game}-review")
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
    assert!(html.contains("href=\"/outlets/\""));
    assert!(html.contains("href=\"/about/\""));
    assert!(!html.contains("href=\"/submit\""));
    assert!(html.contains("lang=\"en\""));
    assert!(!html.contains("role=\"tab\""));
    assert!(html.contains("class=\"site-ad-rail ad-unit\""));
    assert!(html.contains("ca-pub-5490722539415533"));
    assert!(html.contains("data-ad-slot=\"8380670310\""));
    // Loader is injected by the rail gate (once/hour), not as an eager <head> tag.
    assert!(!html.contains(
        "script async src=\"https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js"
    ));
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
                        "raw_score_norm": 80.0,
                        "review_url": "https://www.ign.com/articles/hades-2-review"
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
async fn a_review_without_an_openable_original_does_not_score() {
    let app = app(Database::open_in_memory().unwrap());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/reviews")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "reviews": [
                            {
                                "outlet_slug": "ign",
                                "outlet_name": "IGN",
                                "game_slug": "hades-2",
                                "game_title": "Hades II",
                                "raw_score_norm": 80.0,
                                "review_url": "https://www.ign.com/articles/hades-2-review"
                            },
                            {
                                "outlet_slug": "gamespot",
                                "outlet_name": "GameSpot",
                                "game_slug": "hades-2",
                                "game_title": "Hades II",
                                "raw_score_norm": 100.0
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created = body_json(response).await;
    let game = &created["games"][0];
    assert_eq!(game["review_count"], json!(1));
    assert_eq!(game["agg_score"], json!(80.0));
    assert_eq!(game["simple_mean"], json!(80.0));
}

#[tokio::test]
async fn kotaku_is_listed_without_a_score() {
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
                        "reviews": [
                            {
                                "outlet_slug": "ign",
                                "outlet_name": "IGN",
                                "game_slug": "hades-2",
                                "game_title": "Hades II",
                                "raw_score": 8.0,
                                "scale_max": 10.0,
                                "raw_score_norm": 80.0,
                                "review_url": "https://www.ign.com/articles/hades-2-review"
                            },
                            {
                                "outlet_slug": "kotaku",
                                "outlet_name": "Kotaku",
                                "game_slug": "hades-2",
                                "game_title": "Hades II",
                                "raw_score": 100.0,
                                "scale_max": 100.0,
                                "raw_score_norm": 100.0,
                                "excerpt": "It isn't amazing.",
                                "review_url": "https://kotaku.com/hades-ii-review"
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created = body_json(response).await;
    let game = &created["games"][0];
    assert_eq!(game["review_count"], json!(1));
    assert_eq!(game["agg_score"], json!(80.0));
    let kotaku = game["reviews"]
        .as_array()
        .unwrap()
        .iter()
        .find(|review| review["outlet_slug"] == "kotaku")
        .unwrap();
    assert!(kotaku["raw_score"].is_null());
    assert_eq!(kotaku["excerpt"], json!("It isn't amazing."));

    let page = app
        .oneshot(
            Request::builder()
                .uri("/game/hades-2/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    let bytes = page.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(html.contains("Reviews without a score"));
    assert!(html.contains("It isn&#x27;t amazing.") || html.contains("It isn't amazing."));
    assert!(html.contains("https://kotaku.com/hades-ii-review"));
    assert!(!html.contains("id=\"port-filter\""));
    assert_eq!(html.matches("<div class=\"review\"").count(), 1);
}

#[tokio::test]
async fn port_filter_uses_the_platform_each_review_names() {
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
                        "reviews": [
                            {
                                "outlet_slug": "ign",
                                "outlet_name": "IGN",
                                "game_slug": "resident-evil-4",
                                "game_title": "Resident Evil 4",
                                "platforms": ["ps5", "nintendo switch"],
                                "raw_score_norm": 90.0,
                                "excerpt": "The PS5 version is the one to play.",
                                "review_url": "https://www.ign.com/articles/resident-evil-4-review"
                            },
                            {
                                "outlet_slug": "nintendo-life",
                                "outlet_name": "Nintendo Life",
                                "game_slug": "resident-evil-4",
                                "game_title": "Resident Evil 4",
                                "raw_score_norm": 80.0,
                                "excerpt": "This Switch 2 port holds up.",
                                "review_url": "https://www.nintendolife.com/reviews/resident-evil-4"
                            },
                            {
                                "outlet_slug": "ztgd",
                                "outlet_name": "ZTGD",
                                "game_slug": "resident-evil-4",
                                "game_title": "Resident Evil 4",
                                "raw_score_norm": 85.0,
                                "excerpt": "A landmark remake.",
                                "review_url": "https://ztgd.com/resident-evil-4/review"
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let page = app
        .oneshot(
            Request::builder()
                .uri("/game/resident-evil-4/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    let bytes = page.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(html.contains("id=\"port-filter\""));
    assert!(html.contains("data-port=\"ps5\""));
    assert!(html.contains("data-port=\"switch\""));
    assert!(!html.contains("data-port=\"switch-2\""));
    assert!(html.contains("data-platforms=\"ps5\""));
    assert!(html.contains("data-platforms=\"\""));
    assert!(html.contains("data-platforms=\"ps5 switch\""));
    assert!(!html.contains("data-platforms=\"ps5 switch switch-2\""));
}

#[tokio::test]
async fn ampersand_entity_is_decoded() {
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
                        "outlet_slug": "geeks-a-com",
                        "outlet_name": "Geeks &a; Com",
                        "game_slug": "silksong",
                        "game_title": "Hollow Knight: Silksong",
                        "raw_score_norm": 90.0,
                        "excerpt": "Lost &a; Found is a delight."
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created = body_json(response).await;
    let review = &created["games"][0]["reviews"][0];
    assert_eq!(review["outlet_name"], json!("Geeks & Com"));
    assert_eq!(review["excerpt"], json!("Lost & Found is a delight."));
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
                        "raw_score_norm": 80.0,
                        "review_url": "https://www.ign.com/articles/hades-ii-review"
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
    assert!(outlets.contains("href=\"/outlet/ign/\""));
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
    assert!(outlet.contains("href=\"/game/hades-ii/\""));

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
        .oneshot(
            Request::builder()
                .uri("/?min=0")
                .body(Body::empty())
                .unwrap(),
        )
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
    assert!(home.contains("href=\"/game/hades-ii/\""));
    assert!(home.contains("name=\"platform\""));
    assert!(home.contains("name=\"year\""));
    assert!(home.contains("name=\"min_reviews\""));
    assert!(!home.contains("IGN"));
    assert!(home.contains("href=\"/outlets/\""));
    assert!(home.contains("href=\"/about/\""));
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
    assert!(about.contains("mailto:joeltondbrandao@freedxm.com"));
    assert!(about.contains("https://github.com/freedxmgxd"));
    assert!(about.contains("aria-current=\"page\""));
}

#[tokio::test]
async fn same_url_collapses_and_a_different_url_stays() {
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
                        "reviews": [
                            {
                                "outlet_slug": "ign",
                                "outlet_name": "IGN",
                                "game_slug": "hades-2",
                                "game_title": "Hades II",
                                "raw_score_norm": 80.0,
                                "review_url": "https://www.ign.com/articles/hades-2-review?utm=1#top"
                            },
                            {
                                "outlet_slug": "ign",
                                "outlet_name": "IGN",
                                "game_slug": "hades-2",
                                "game_title": "Hades II",
                                "raw_score_norm": 70.0,
                                "review_url": "https://ign.com/articles/hades-2-review/"
                            },
                            {
                                "outlet_slug": "ign",
                                "outlet_name": "IGN",
                                "game_slug": "hades-2",
                                "game_title": "Hades II",
                                "raw_score_norm": 90.0,
                                "review_url": "https://ign.com/articles/hades-2-ps5-review"
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created = body_json(response).await;
    let reviews = created["games"][0]["reviews"].as_array().unwrap();
    assert_eq!(reviews.len(), 2, "{reviews:?}");
    let urls: Vec<&str> = reviews
        .iter()
        .filter_map(|review| review["review_url"].as_str())
        .collect();
    assert!(urls.iter().any(|url| url.contains("hades-2-review")));
    assert!(urls.iter().any(|url| url.contains("hades-2-ps5-review")));
}
