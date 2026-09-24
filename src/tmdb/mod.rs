pub mod images;
pub mod types;

use std::collections::HashMap;

use clap::ValueEnum;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http;
use types::{Collection, MovieDetail, SearchResults, SeasonDetail, TvShowDetail};

const API_BASE: &str = "https://api.themoviedb.org/3";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MediaType {
    Movie,
    Tv,
}

enum Auth {
    /// v3 API key, sent as a query parameter.
    ApiKey(String),
    /// v4 read access token (a JWT), sent as a bearer header.
    Bearer(String),
}

pub struct TmdbClient {
    http: Client,
    auth: Auth,
    fallback_language: String,
    image_languages: Vec<String>,
}

impl TmdbClient {
    pub fn new(http: Client, config: &Config) -> Result<Self> {
        let key = config.api_key()?.to_string();
        let auth = if key.starts_with("eyJ") {
            Auth::Bearer(key)
        } else {
            Auth::ApiKey(key)
        };
        Ok(Self {
            http,
            auth,
            fallback_language: config.defaults.fallback_language.clone(),
            image_languages: config.image_languages(),
        })
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let mut req = self.http.get(format!("{API_BASE}{path}")).query(query);
        req = match &self.auth {
            Auth::ApiKey(key) => req.query(&[("api_key", key)]),
            Auth::Bearer(token) => req.bearer_auth(token),
        };

        let resp = http::send(req).await?;
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Err(Error::NotFound(format!(
                "Resource not found on TMDB: {path}"
            )));
        }
        let body = resp
            .bytes()
            .await
            .map_err(|e| Error::Network(e.without_url()))?;
        if !status.is_success() {
            return Err(Error::Api(format!(
                "TMDB API error {status}: {}",
                error_message(&body)
            )));
        }
        serde_json::from_slice(&body)
            .map_err(|e| Error::Api(format!("Unexpected TMDB response for {path}: {e}")))
    }

    pub async fn search(
        &self,
        query: &str,
        media_type: MediaType,
        year: Option<u32>,
        language: &str,
    ) -> Result<SearchResults> {
        let (path, year_param) = match media_type {
            MediaType::Movie => ("/search/movie", "year"),
            MediaType::Tv => ("/search/tv", "first_air_date_year"),
        };
        let year = year.map(|y| y.to_string());
        let mut params = vec![("query", query), ("language", language)];
        if let Some(y) = &year {
            params.push((year_param, y));
        }
        self.get(path, &params).await
    }

    /// Full movie details. Overview and tagline fall back to the configured
    /// fallback language when the requested language has no overview.
    pub async fn movie(&self, id: u64, language: &str) -> Result<MovieDetail> {
        let path = format!("/movie/{id}");
        let (images, videos) = (self.image_filter(language), video_filter(language));
        let mut movie: MovieDetail = self
            .get(
                &path,
                &[
                    ("language", language),
                    (
                        "append_to_response",
                        "credits,videos,images,release_dates,keywords",
                    ),
                    ("include_image_language", &images),
                    ("include_video_language", &videos),
                ],
            )
            .await?;

        if let Some(fallback) = self.fallback_for(language)
            && is_blank(movie.overview.as_deref())
            && let Ok(alt) = self
                .get::<MovieDetail>(&path, &[("language", fallback)])
                .await
        {
            fill_blank(&mut movie.overview, alt.overview);
            fill_blank(&mut movie.tagline, alt.tagline);
        }
        Ok(movie)
    }

    pub async fn collection(&self, id: u64, language: &str) -> Result<Collection> {
        self.get(&format!("/collection/{id}"), &[("language", language)])
            .await
    }

    /// Full show details, with overview falling back like [`Self::movie`].
    pub async fn tv_show(&self, id: u64, language: &str) -> Result<TvShowDetail> {
        let path = format!("/tv/{id}");
        let (images, videos) = (self.image_filter(language), video_filter(language));
        let mut show: TvShowDetail = self
            .get(
                &path,
                &[
                    ("language", language),
                    (
                        "append_to_response",
                        "credits,videos,images,content_ratings,external_ids,keywords",
                    ),
                    ("include_image_language", &images),
                    ("include_video_language", &videos),
                ],
            )
            .await?;

        if let Some(fallback) = self.fallback_for(language)
            && is_blank(show.overview.as_deref())
            && let Ok(alt) = self
                .get::<TvShowDetail>(&path, &[("language", fallback)])
                .await
        {
            fill_blank(&mut show.overview, alt.overview);
        }
        Ok(show)
    }

    pub async fn season(&self, tv_id: u64, season: u32, language: &str) -> Result<SeasonDetail> {
        let images = self.image_filter(language);
        self.get(
            &format!("/tv/{tv_id}/season/{season}"),
            &[
                ("language", language),
                ("append_to_response", "images"),
                ("include_image_language", &images),
            ],
        )
        .await
    }

    /// Episode number → episode title for one season, without any appended data.
    pub async fn episode_titles(
        &self,
        tv_id: u64,
        season: u32,
        language: &str,
    ) -> Result<HashMap<u32, String>> {
        let detail: SeasonDetail = self
            .get(
                &format!("/tv/{tv_id}/season/{season}"),
                &[("language", language)],
            )
            .await?;
        Ok(detail
            .episodes
            .into_iter()
            .filter_map(|ep| ep.name.map(|name| (ep.episode_number, name)))
            .collect())
    }

    fn fallback_for(&self, language: &str) -> Option<&str> {
        let fallback = self.fallback_language.as_str();
        (!fallback.is_empty() && fallback != language).then_some(fallback)
    }

    /// `include_image_language` value: preferred image languages, the request
    /// language, then language-neutral images.
    fn image_filter(&self, language: &str) -> String {
        let mut langs: Vec<&str> = self.image_languages.iter().map(String::as_str).collect();
        langs.extend([primary_subtag(language), "null"]);
        dedup_join(langs)
    }
}

pub fn primary_subtag(language: &str) -> &str {
    language.split('-').next().unwrap_or(language)
}

/// `include_video_language` value: trailers are often only published in English.
fn video_filter(language: &str) -> String {
    dedup_join(vec![primary_subtag(language), "en", "null"])
}

fn dedup_join(items: Vec<&str>) -> String {
    let mut seen = Vec::with_capacity(items.len());
    for item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    seen.join(",")
}

fn is_blank(value: Option<&str>) -> bool {
    value.is_none_or(|s| s.trim().is_empty())
}

fn fill_blank(target: &mut Option<String>, alt: Option<String>) {
    if is_blank(target.as_deref()) && !is_blank(alt.as_deref()) {
        *target = alt;
    }
}

fn error_message(body: &[u8]) -> String {
    #[derive(serde::Deserialize)]
    struct TmdbError {
        status_message: String,
    }
    serde_json::from_slice::<TmdbError>(body).map_or_else(
        |_| String::from_utf8_lossy(body).into_owned(),
        |e| e.status_message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_language_filters() {
        assert_eq!(primary_subtag("zh-CN"), "zh");
        assert_eq!(primary_subtag("en"), "en");
        assert_eq!(video_filter("zh-CN"), "zh,en,null");
        assert_eq!(video_filter("en-US"), "en,null");
        assert_eq!(dedup_join(vec!["en", "zh", "en", "null"]), "en,zh,null");
    }

    #[test]
    fn extracts_tmdb_error_message() {
        let body = br#"{"status_code":7,"status_message":"Invalid API key","success":false}"#;
        assert_eq!(error_message(body), "Invalid API key");
        assert_eq!(error_message(b"oops"), "oops");
    }
}
