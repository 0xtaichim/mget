pub mod types;

use crate::config::Config;
use crate::error::{Error, Result};
use reqwest::Client;
use std::time::Duration;

pub struct TmdbClient {
    client: Client,
    api_key: String,
    language: String,
    fallback_language: String,
}

impl TmdbClient {
    pub fn new(config: &Config) -> Result<Self> {
        let api_key = config.api_key()?.to_string();
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(config.network.timeout));
        if let Some(proxy) = &config.network.proxy {
            builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(Error::Network)?);
        }
        let client = builder.build().map_err(Error::Network)?;
        Ok(Self {
            client,
            api_key,
            language: config.defaults.language.clone(),
            fallback_language: config.defaults.fallback_language.clone(),
        })
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let mut retries = 0;
        loop {
            let resp = self.client.get(url).send().await;
            match resp {
                Ok(r) => {
                    if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        if retries < 3 {
                            retries += 1;
                            let wait = Duration::from_millis(500 * retries as u64);
                            tokio::time::sleep(wait).await;
                            continue;
                        }
                        return Err(Error::Api("Rate limited by TMDB".into()));
                    }
                    if r.status() == reqwest::StatusCode::NOT_FOUND {
                        return Err(Error::NotFound("Resource not found on TMDB".into()));
                    }
                    if !r.status().is_success() {
                        let status = r.status();
                        let body = r.text().await.unwrap_or_default();
                        return Err(Error::Api(format!("TMDB API error {}: {}", status, body)));
                    }
                    return r.json::<T>().await.map_err(Error::Network);
                }
                Err(e) => {
                    if retries < 3 {
                        retries += 1;
                        let wait = Duration::from_secs(retries as u64);
                        tokio::time::sleep(wait).await;
                        continue;
                    }
                    return Err(Error::Network(e));
                }
            }
        }
    }

    pub async fn search(
        &self,
        query: &str,
        media_type: &str,
        year: Option<u32>,
        language: Option<&str>,
    ) -> Result<types::SearchResults> {
        let lang = language.unwrap_or(&self.language);
        let endpoint = match media_type {
            "movie" => "movie",
            "tv" => "tv",
            _ => return Err(Error::Parse(format!("Unknown media type: {}", media_type))),
        };
        let mut url = url::Url::parse(&format!(
            "https://api.themoviedb.org/3/search/{}",
            endpoint
        ))
        .map_err(|e| Error::Parse(e.to_string()))?;
        url.query_pairs_mut()
            .append_pair("api_key", &self.api_key)
            .append_pair("query", query)
            .append_pair("language", lang);
        if let Some(y) = year {
            match media_type {
                "movie" => {
                    url.query_pairs_mut()
                        .append_pair("year", &y.to_string());
                }
                "tv" => {
                    url.query_pairs_mut()
                        .append_pair("first_air_date_year", &y.to_string());
                }
                _ => {}
            }
        }
        self.get_json(url.as_str()).await
    }

    pub async fn get_movie(
        &self,
        id: u64,
        language: Option<&str>,
    ) -> Result<types::MovieDetail> {
        let lang = language.unwrap_or(&self.language);
        let url = format!(
            "https://api.themoviedb.org/3/movie/{}?api_key={}&language={}&append_to_response=credits,videos,images,release_dates,keywords&include_image_language={},null",
            id, self.api_key, lang, lang.split('-').next().unwrap_or("en"),
        );
        self.get_json(&url).await
    }

    pub async fn get_movie_fallback(
        &self,
        id: u64,
        language: Option<&str>,
    ) -> Result<types::MovieDetail> {
        let lang = language.unwrap_or(&self.language);
        let mut movie = self.get_movie(id, Some(lang)).await?;

        if movie.overview.as_deref().unwrap_or("").is_empty() && self.fallback_language != lang {
            if let Ok(fallback) = self.get_movie(id, Some(&self.fallback_language)).await {
                if movie.overview.as_deref().unwrap_or("").is_empty() {
                    movie.overview = fallback.overview;
                }
                if movie.tagline.as_deref().unwrap_or("").is_empty() {
                    movie.tagline = fallback.tagline;
                }
            }
        }
        Ok(movie)
    }

    pub async fn get_collection(&self, id: u64) -> Result<types::Collection> {
        let url = format!(
            "https://api.themoviedb.org/3/collection/{}?api_key={}&language={}",
            id, self.api_key, self.language,
        );
        self.get_json(&url).await
    }

    pub async fn get_tv_show(
        &self,
        id: u64,
        language: Option<&str>,
    ) -> Result<types::TvShowDetail> {
        let lang = language.unwrap_or(&self.language);
        let url = format!(
            "https://api.themoviedb.org/3/tv/{}?api_key={}&language={}&append_to_response=credits,videos,images,content_ratings,external_ids,keywords&include_image_language={},null",
            id, self.api_key, lang, lang.split('-').next().unwrap_or("en"),
        );
        self.get_json(&url).await
    }

    pub async fn get_tv_show_fallback(
        &self,
        id: u64,
        language: Option<&str>,
    ) -> Result<types::TvShowDetail> {
        let lang = language.unwrap_or(&self.language);
        let mut show = self.get_tv_show(id, Some(lang)).await?;
        if show.overview.as_deref().unwrap_or("").is_empty() && self.fallback_language != lang {
            if let Ok(fallback) = self.get_tv_show(id, Some(&self.fallback_language)).await {
                if show.overview.as_deref().unwrap_or("").is_empty() {
                    show.overview = fallback.overview;
                }
            }
        }
        Ok(show)
    }

    pub async fn get_tv_season(
        &self,
        tv_id: u64,
        season_number: u32,
        language: Option<&str>,
    ) -> Result<types::SeasonDetail> {
        let lang = language.unwrap_or(&self.language);
        let url = format!(
            "https://api.themoviedb.org/3/tv/{}/season/{}?api_key={}&language={}&append_to_response=credits,videos,images&include_image_language={},null",
            tv_id, season_number, self.api_key, lang, lang.split('-').next().unwrap_or("en"),
        );
        self.get_json(&url).await
    }
}
