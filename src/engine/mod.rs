//! Motor estatístico puro. Sem I/O, sem HTTP, sem banco.

mod scoring;

pub use scoring::{
    C, GameScore, ReviewInput, SIGMA_MIN, SIGMA_TARGET, ScoredReview, ScoringError,
    ScoringResult, map_z, mean_and_population_std, normalize_score, score_corpus,
};
