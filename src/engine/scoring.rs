//! Normalização para 0–100 e regularização bayesiana por veículo.
//!
//! O desvio-padrão é populacional (divisor `n`). Com uma review o desvio
//! amostral (`n - 1`) é indefinido; nesse caso `σ = 0` e o piso `σ_min`
//! junto com o shrinkage impedem Z-score infinito.
//!
//! `μ_j` e `σ_j` incluem todas as reviews do veículo, inclusive a que está
//! sendo pontuada. Um jogo novo altera `μ_global` e `σ_global`, então o
//! recálculo correto é sempre o corpus inteiro.

use std::collections::BTreeMap;

pub const C: f64 = 30.0;
pub const SIGMA_MIN: f64 = 5.0;
/// Width of the published scale: each +1 in Z moves a mapped score by this many points.
pub const SIGMA_TARGET: f64 = 10.0;

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewInput {
    pub outlet_key: String,
    pub game_key: String,
    pub raw_score_norm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutletStats {
    pub outlet_key: String,
    pub n: usize,
    pub mu: f64,
    pub sigma: f64,
    pub mu_prime: f64,
    pub sigma_prime: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScoredReview {
    pub outlet_key: String,
    pub game_key: String,
    pub raw_score_norm: f64,
    pub z_score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameScore {
    pub game_key: String,
    pub agg_score: f64,
    pub simple_mean: f64,
    pub review_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScoringResult {
    pub mu_global: f64,
    pub sigma_global: f64,
    pub outlets: Vec<OutletStats>,
    pub reviews: Vec<ScoredReview>,
    pub games: Vec<GameScore>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScoringError {
    NonFinite,
    OutOfRange,
}

impl std::fmt::Display for ScoringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScoringError::NonFinite => write!(f, "score is not finite"),
            ScoringError::OutOfRange => write!(f, "score outside 0..=100"),
        }
    }
}

impl std::error::Error for ScoringError {}

/// Converte uma nota bruta para a escala contínua 0–100.
/// `4/5` → `80`, `8.5/10` → `85`. Não aplica clamp.
pub fn normalize_score(raw: f64, scale_max: f64) -> Result<f64, ScoringError> {
    if !raw.is_finite() || !scale_max.is_finite() || scale_max <= 0.0 {
        return Err(ScoringError::NonFinite);
    }
    let norm = raw / scale_max * 100.0;
    if !norm.is_finite() {
        return Err(ScoringError::NonFinite);
    }
    Ok(norm)
}

/// Média e desvio-padrão populacional. Corpus vazio devolve `(0, 0)`.
pub fn mean_and_population_std(values: &[f64]) -> (f64, f64) {
    let n = values.len();
    if n == 0 {
        return (0.0, 0.0);
    }
    let mu = values.iter().sum::<f64>() / n as f64;
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mu;
            delta * delta
        })
        .sum::<f64>()
        / n as f64;
    (mu, variance.sqrt())
}

fn shrink(n: usize, stat: f64, global: f64) -> f64 {
    let n = n as f64;
    (n * stat + C * global) / (n + C)
}

fn regularize(n: usize, mu: f64, sigma: f64, mu_global: f64, sigma_global: f64) -> (f64, f64) {
    let mu_prime = shrink(n, mu, mu_global);
    let sigma_prime = shrink(n, sigma, sigma_global).max(SIGMA_MIN);
    (mu_prime, sigma_prime)
}

fn map_z(z: f64, mu_global: f64) -> f64 {
    (mu_global + z * SIGMA_TARGET).clamp(0.0, 100.0)
}

/// Converte cada Z para 0–100 centrado em `μ_global` e só então tira a média.
fn aggregate(z_scores: &[f64], mu_global: f64) -> f64 {
    let mapped: f64 = z_scores.iter().map(|&z| map_z(z, mu_global)).sum();
    mapped / z_scores.len() as f64
}

/// Recalcula parâmetros globais, shrinkage de cada veículo, Z-scores e
/// a nota agregada de cada jogo. Entradas duplicadas `(veículo, jogo)`
/// ficam com a última nota. O resultado não depende da ordem.
pub fn score_corpus(reviews: &[ReviewInput]) -> Result<ScoringResult, ScoringError> {
    let mut deduped: BTreeMap<(String, String), f64> = BTreeMap::new();
    for review in reviews {
        if !review.raw_score_norm.is_finite() {
            return Err(ScoringError::NonFinite);
        }
        if !(0.0..=100.0).contains(&review.raw_score_norm) {
            return Err(ScoringError::OutOfRange);
        }
        deduped.insert(
            (review.outlet_key.clone(), review.game_key.clone()),
            review.raw_score_norm,
        );
    }

    let canonical: Vec<ReviewInput> = deduped
        .into_iter()
        .map(|((outlet_key, game_key), raw_score_norm)| ReviewInput {
            outlet_key,
            game_key,
            raw_score_norm,
        })
        .collect();

    let norms: Vec<f64> = canonical
        .iter()
        .map(|review| review.raw_score_norm)
        .collect();
    let (mu_global, sigma_global) = mean_and_population_std(&norms);

    let mut by_outlet: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for review in &canonical {
        by_outlet
            .entry(review.outlet_key.as_str())
            .or_default()
            .push(review.raw_score_norm);
    }

    let mut outlet_stats: BTreeMap<String, OutletStats> = BTreeMap::new();
    for (outlet_key, scores) in by_outlet {
        let (mu, sigma) = mean_and_population_std(&scores);
        let (mu_prime, sigma_prime) = regularize(scores.len(), mu, sigma, mu_global, sigma_global);
        outlet_stats.insert(
            outlet_key.to_string(),
            OutletStats {
                outlet_key: outlet_key.to_string(),
                n: scores.len(),
                mu,
                sigma,
                mu_prime,
                sigma_prime,
            },
        );
    }

    let scored = canonical
        .iter()
        .map(|review| {
            let stats = &outlet_stats[&review.outlet_key];
            let z_score = (review.raw_score_norm - stats.mu_prime) / stats.sigma_prime;
            ScoredReview {
                outlet_key: review.outlet_key.clone(),
                game_key: review.game_key.clone(),
                raw_score_norm: review.raw_score_norm,
                z_score,
            }
        })
        .collect::<Vec<_>>();

    let mut by_game: BTreeMap<&str, Vec<&ScoredReview>> = BTreeMap::new();
    for review in &scored {
        by_game
            .entry(review.game_key.as_str())
            .or_default()
            .push(review);
    }

    let games = by_game
        .into_iter()
        .map(|(game_key, game_reviews)| {
            let z_scores: Vec<f64> = game_reviews.iter().map(|review| review.z_score).collect();
            let simple_mean = game_reviews
                .iter()
                .map(|review| review.raw_score_norm)
                .sum::<f64>()
                / game_reviews.len() as f64;
            GameScore {
                game_key: game_key.to_string(),
                agg_score: aggregate(&z_scores, mu_global),
                simple_mean,
                review_count: game_reviews.len(),
            }
        })
        .collect();

    Ok(ScoringResult {
        mu_global,
        sigma_global,
        outlets: outlet_stats.into_values().collect(),
        reviews: scored,
        games,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        let diff = (actual - expected).abs();
        assert!(diff < 1e-9, "{actual} != {expected} (diff {diff})");
    }

    fn review(outlet: &str, game: &str, score: f64) -> ReviewInput {
        ReviewInput {
            outlet_key: outlet.to_string(),
            game_key: game.to_string(),
            raw_score_norm: score,
        }
    }

    fn z_of(result: &ScoringResult, outlet: &str, game: &str) -> f64 {
        result
            .reviews
            .iter()
            .find(|review| review.outlet_key == outlet && review.game_key == game)
            .unwrap()
            .z_score
    }

    fn game_of<'a>(result: &'a ScoringResult, game: &str) -> &'a GameScore {
        result
            .games
            .iter()
            .find(|item| item.game_key == game)
            .unwrap()
    }

    fn outlet_of<'a>(result: &'a ScoringResult, outlet: &str) -> &'a OutletStats {
        result
            .outlets
            .iter()
            .find(|item| item.outlet_key == outlet)
            .unwrap()
    }

    #[test]
    fn normalizes_common_scales() {
        close(normalize_score(4.0, 5.0).unwrap(), 80.0);
        close(normalize_score(8.5, 10.0).unwrap(), 85.0);
        assert_eq!(normalize_score(8.0, 0.0), Err(ScoringError::NonFinite));
    }

    #[test]
    fn single_review_shrinks_to_target_mean() {
        let result = score_corpus(&[review("ign", "hades", 80.0)]).unwrap();

        close(result.mu_global, 80.0);
        close(result.sigma_global, 0.0);

        let outlet = outlet_of(&result, "ign");
        assert_eq!(outlet.n, 1);
        close(outlet.mu, 80.0);
        close(outlet.sigma, 0.0);
        close(outlet.mu_prime, 80.0);
        close(outlet.sigma_prime, SIGMA_MIN);
        close(z_of(&result, "ign", "hades"), 0.0);

        let game = game_of(&result, "hades");
        close(game.agg_score, 80.0); // Z=0 maps to μ_global
        close(game.simple_mean, 80.0);
        assert_eq!(game.review_count, 1);
    }

    #[test]
    fn identical_corpus_has_zero_z_and_global_mean_score() {
        let result = score_corpus(&[
            review("a", "g1", 62.0),
            review("a", "g2", 62.0),
            review("b", "g3", 62.0),
        ])
        .unwrap();

        for review in &result.reviews {
            close(review.z_score, 0.0);
        }
        for game in &result.games {
            close(game.agg_score, result.mu_global);
        }
        for outlet in &result.outlets {
            close(outlet.sigma, 0.0);
            close(outlet.sigma_prime, SIGMA_MIN);
        }
    }

    #[test]
    fn zero_variance_outlet_uses_shrunk_sigma_above_floor() {
        let result = score_corpus(&[
            review("steady", "g1", 90.0),
            review("steady", "g2", 90.0),
            review("steady", "g3", 90.0),
            review("other", "g4", 50.0),
        ])
        .unwrap();

        let steady = outlet_of(&result, "steady");
        close(steady.sigma, 0.0);
        close(steady.mu_prime, 890.0 / 11.0);
        close(steady.sigma_prime, 100.0 * 3.0_f64.sqrt() / 11.0);
        assert!(steady.sigma_prime > SIGMA_MIN);
        close(z_of(&result, "steady", "g1"), 1.0 / 3.0_f64.sqrt());
        close(
            game_of(&result, "g1").agg_score,
            result.mu_global + SIGMA_TARGET / 3.0_f64.sqrt(),
        );

        let other = outlet_of(&result, "other");
        close(other.sigma, 0.0);
        close(other.sigma_prime, 300.0 * 3.0_f64.sqrt() / 31.0);
        close(z_of(&result, "other", "g4"), -3.0_f64.sqrt());
        close(
            game_of(&result, "g4").agg_score,
            result.mu_global - SIGMA_TARGET * 3.0_f64.sqrt(),
        );
    }

    #[test]
    fn sigma_floor_when_shrunk_deviation_is_tiny() {
        let mut reviews = Vec::new();
        for index in 0..100 {
            reviews.push(review("flat", &format!("g{index}"), 50.0));
        }
        reviews.push(review("nudge", "other", 51.0));

        let norms: Vec<f64> = reviews.iter().map(|review| review.raw_score_norm).collect();
        let (_, sigma_global) = mean_and_population_std(&norms);
        let n = 100.0;
        let shrunk = (n * 0.0 + C * sigma_global) / (n + C);
        assert!(
            shrunk < SIGMA_MIN,
            "shrunk sigma {shrunk} should sit under the floor"
        );

        let result = score_corpus(&reviews).unwrap();
        let flat = outlet_of(&result, "flat");
        close(flat.sigma, 0.0);
        close(flat.sigma_prime, SIGMA_MIN);
        assert!(z_of(&result, "flat", "g0").is_finite());
    }

    #[test]
    fn same_raw_score_harsh_outlet_ranks_above_lenient_one() {
        let mut reviews = vec![
            review("harsh", "shared", 70.0),
            review("lenient", "shared", 70.0),
        ];
        // C = 30. Com 3 reviews o prior global engole o viés; 60 fixam o histórico.
        for index in 0..60 {
            reviews.push(review("harsh", &format!("h{index}"), 40.0));
            reviews.push(review("lenient", &format!("l{index}"), 90.0));
        }

        let result = score_corpus(&reviews).unwrap();
        let harsh_z = z_of(&result, "harsh", "shared");
        let lenient_z = z_of(&result, "lenient", "shared");

        assert!(harsh_z > 0.0, "z carrasco = {harsh_z}");
        assert!(lenient_z < 0.0, "z bonzinho = {lenient_z}");
        assert!(harsh_z > lenient_z);

        let shared = game_of(&result, "shared");
        close(shared.simple_mean, 70.0);
        close(
            shared.agg_score,
            (map_z(harsh_z, result.mu_global) + map_z(lenient_z, result.mu_global)) / 2.0,
        );
    }

    #[test]
    fn clamps_high_and_low_extremes() {
        // Many near-identical scores keep σ' near the floor so an outlier
        // still maps past 0–100 when centered on μ_global.
        let mut high = Vec::new();
        for index in 0..100 {
            high.push(review("steady", &format!("filler-{index}"), 50.0));
        }
        high.push(review("steady", "masterpiece", 100.0));
        let high_result = score_corpus(&high).unwrap();
        let high_z = z_of(&high_result, "steady", "masterpiece");
        let unclamped_high = high_result.mu_global + high_z * SIGMA_TARGET;
        assert!(unclamped_high > 100.0, "unclamped {unclamped_high}");
        close(game_of(&high_result, "masterpiece").agg_score, 100.0);

        let mut low = Vec::new();
        for index in 0..100 {
            low.push(review("steady", &format!("filler-{index}"), 50.0));
        }
        low.push(review("steady", "dud", 0.0));
        let low_result = score_corpus(&low).unwrap();
        let low_z = z_of(&low_result, "steady", "dud");
        let unclamped_low = low_result.mu_global + low_z * SIGMA_TARGET;
        assert!(unclamped_low < 0.0, "unclamped {unclamped_low}");
        close(game_of(&low_result, "dud").agg_score, 0.0);
    }

    #[test]
    fn duplicate_pair_keeps_last_score_and_is_order_independent() {
        let first =
            score_corpus(&[review("ign", "hades", 10.0), review("ign", "hades", 80.0)]).unwrap();
        let second = score_corpus(&[
            review("ign", "hades", 80.0),
            review("polygon", "hades", 70.0),
        ])
        .unwrap();
        let reversed = score_corpus(&[
            review("polygon", "hades", 70.0),
            review("ign", "hades", 80.0),
        ])
        .unwrap();

        assert_eq!(first.reviews.len(), 1);
        close(first.reviews[0].raw_score_norm, 80.0);
        assert_eq!(second, reversed);
    }

    #[test]
    fn rejects_non_finite_and_out_of_range() {
        assert_eq!(
            score_corpus(&[review("ign", "hades", f64::NAN)]),
            Err(ScoringError::NonFinite)
        );
        assert_eq!(
            score_corpus(&[review("ign", "hades", 100.1)]),
            Err(ScoringError::OutOfRange)
        );
        assert_eq!(
            score_corpus(&[review("ign", "hades", -0.1)]),
            Err(ScoringError::OutOfRange)
        );
    }
}
