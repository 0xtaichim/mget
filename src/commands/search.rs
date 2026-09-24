use serde::Serialize;

use crate::cli::SearchArgs;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::http;
use crate::tmdb::{TmdbClient, images};

#[derive(Serialize)]
struct SearchOutput {
    results: Vec<SearchHit>,
    total_results: u32,
}

#[derive(Serialize)]
struct SearchHit {
    id: u64,
    title: Option<String>,
    original_title: Option<String>,
    year: Option<String>,
    overview: Option<String>,
    poster_url: Option<String>,
    vote_average: Option<f64>,
}

pub async fn run(args: SearchArgs) -> Result<()> {
    let config = Config::load()?;
    let tmdb = TmdbClient::new(http::build_client(&config.network)?, &config)?;
    let lang = args.lang.as_deref().unwrap_or(&config.defaults.language);
    let found = tmdb
        .search(&args.query, args.media_type, args.year, lang)
        .await?;

    if found.results.is_empty() {
        return Err(Error::NotFound(format!(
            "No results found for '{}'",
            args.query
        )));
    }

    let results = found
        .results
        .into_iter()
        .map(|r| SearchHit {
            id: r.id,
            year: r
                .release_date
                .as_deref()
                .and_then(|d| d.get(..4))
                .map(String::from),
            poster_url: r.poster_path.as_deref().map(|p| images::url("w500", p)),
            title: r.title,
            original_title: r.original_title,
            overview: r.overview,
            vote_average: r.vote_average,
        })
        .collect();

    super::print_json(&SearchOutput {
        results,
        total_results: found.total_results,
    })
}
