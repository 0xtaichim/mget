//! TMDB response models, limited to the fields mget consumes.

use serde::{Deserialize, Deserializer};

/// Treats an explicit `null` like a missing field.
fn nullable<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(d).map(Option::unwrap_or_default)
}

#[derive(Debug, Deserialize)]
pub struct SearchResults {
    pub total_results: u32,
    #[serde(default, deserialize_with = "nullable")]
    pub results: Vec<SearchResult>,
}

/// A movie or TV search hit; TV field names are aliased onto the movie ones.
#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub id: u64,
    #[serde(alias = "name")]
    pub title: Option<String>,
    #[serde(alias = "original_name")]
    pub original_title: Option<String>,
    #[serde(alias = "first_air_date")]
    pub release_date: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub vote_average: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct MovieDetail {
    pub id: u64,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub release_date: Option<String>,
    pub runtime: Option<u32>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    pub imdb_id: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub genres: Vec<Named>,
    #[serde(default, deserialize_with = "nullable")]
    pub production_companies: Vec<Named>,
    #[serde(default, deserialize_with = "nullable")]
    pub production_countries: Vec<Named>,
    #[serde(default, deserialize_with = "nullable")]
    pub spoken_languages: Vec<SpokenLanguage>,
    pub belongs_to_collection: Option<CollectionRef>,
    pub credits: Option<Credits>,
    pub videos: Option<VideoResults>,
    pub images: Option<ImageResults>,
    pub release_dates: Option<ReleaseDateResults>,
    pub keywords: Option<MovieKeywords>,
}

/// Any TMDB object where only the display name matters (genres, studios, countries).
#[derive(Debug, Deserialize)]
pub struct Named {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct SpokenLanguage {
    pub name: String,
    pub english_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CollectionRef {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Collection {
    pub id: u64,
    pub name: String,
    pub overview: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Credits {
    #[serde(default, deserialize_with = "nullable")]
    pub cast: Vec<CastMember>,
    #[serde(default, deserialize_with = "nullable")]
    pub crew: Vec<CrewMember>,
}

#[derive(Debug, Deserialize)]
pub struct CastMember {
    pub id: u64,
    pub name: String,
    pub character: Option<String>,
    pub order: Option<u32>,
    pub profile_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CrewMember {
    pub id: u64,
    pub name: String,
    pub job: String,
}

#[derive(Debug, Deserialize)]
pub struct VideoResults {
    #[serde(default, deserialize_with = "nullable")]
    pub results: Vec<Video>,
}

#[derive(Debug, Deserialize)]
pub struct Video {
    pub key: String,
    pub site: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct ImageResults {
    #[serde(default, deserialize_with = "nullable")]
    pub posters: Vec<Image>,
    #[serde(default, deserialize_with = "nullable")]
    pub backdrops: Vec<Image>,
    #[serde(default, deserialize_with = "nullable")]
    pub logos: Vec<Image>,
}

#[derive(Debug, Deserialize)]
pub struct Image {
    pub file_path: String,
    pub width: Option<u32>,
    pub vote_average: Option<f64>,
    pub iso_639_1: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseDateResults {
    #[serde(default, deserialize_with = "nullable")]
    pub results: Vec<ReleaseDateCountry>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseDateCountry {
    pub iso_3166_1: String,
    #[serde(default, deserialize_with = "nullable")]
    pub release_dates: Vec<ReleaseDate>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseDate {
    pub certification: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MovieKeywords {
    #[serde(default, deserialize_with = "nullable")]
    pub keywords: Vec<Named>,
}

#[derive(Debug, Deserialize)]
pub struct TvShowDetail {
    pub id: u64,
    pub name: Option<String>,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub first_air_date: Option<String>,
    pub status: Option<String>,
    pub original_language: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    #[serde(default, deserialize_with = "nullable")]
    pub episode_run_time: Vec<u32>,
    #[serde(default, deserialize_with = "nullable")]
    pub genres: Vec<Named>,
    #[serde(default, deserialize_with = "nullable")]
    pub production_companies: Vec<Named>,
    #[serde(default, deserialize_with = "nullable")]
    pub origin_country: Vec<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub seasons: Vec<SeasonSummary>,
    #[serde(default, deserialize_with = "nullable")]
    pub networks: Vec<Named>,
    pub credits: Option<Credits>,
    pub videos: Option<VideoResults>,
    pub images: Option<ImageResults>,
    pub content_ratings: Option<ContentRatings>,
    pub external_ids: Option<ExternalIds>,
    pub keywords: Option<TvKeywords>,
}

#[derive(Debug, Deserialize)]
pub struct SeasonSummary {
    pub name: Option<String>,
    pub season_number: u32,
    pub poster_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ContentRatings {
    #[serde(default, deserialize_with = "nullable")]
    pub results: Vec<ContentRating>,
}

#[derive(Debug, Deserialize)]
pub struct ContentRating {
    pub iso_3166_1: String,
    pub rating: String,
}

#[derive(Debug, Deserialize)]
pub struct ExternalIds {
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct TvKeywords {
    #[serde(default, deserialize_with = "nullable")]
    pub results: Vec<Named>,
}

#[derive(Debug, Deserialize)]
pub struct SeasonDetail {
    pub poster_path: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub episodes: Vec<Episode>,
    pub images: Option<ImageResults>,
}

#[derive(Debug, Deserialize)]
pub struct Episode {
    pub id: u64,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub episode_number: u32,
    pub season_number: u32,
    pub air_date: Option<String>,
    pub still_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    pub runtime: Option<u32>,
    #[serde(default, deserialize_with = "nullable")]
    pub crew: Vec<CrewMember>,
    #[serde(default, deserialize_with = "nullable")]
    pub guest_stars: Vec<CastMember>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_collections_deserialize_as_empty() {
        let ep: Episode = serde_json::from_str(
            r#"{"id":1,"episode_number":2,"season_number":1,"crew":null,"guest_stars":null}"#,
        )
        .unwrap();
        assert!(ep.crew.is_empty());
        assert!(ep.guest_stars.is_empty());
    }

    #[test]
    fn tv_search_fields_alias_onto_movie_fields() {
        let r: SearchResult = serde_json::from_str(
            r#"{"id":1,"name":"Show","original_name":"Orig","first_air_date":"2020-01-01"}"#,
        )
        .unwrap();
        assert_eq!(r.title.as_deref(), Some("Show"));
        assert_eq!(r.original_title.as_deref(), Some("Orig"));
        assert_eq!(r.release_date.as_deref(), Some("2020-01-01"));
    }
}
