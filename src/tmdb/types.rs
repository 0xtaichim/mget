use serde::{Deserialize, Serialize};

// Search results
#[derive(Debug, Deserialize, Serialize)]
pub struct SearchResults {
    pub page: u32,
    pub total_results: u32,
    pub total_pages: u32,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Deserialize, Serialize)]
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
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    pub popularity: Option<f64>,
    #[serde(default)]
    pub genre_ids: Vec<u32>,
    pub media_type: Option<String>,
    pub original_language: Option<String>,
}

// Movie detail
#[derive(Debug, Deserialize, Serialize)]
pub struct MovieDetail {
    pub id: u64,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub release_date: Option<String>,
    pub runtime: Option<u32>,
    pub budget: Option<u64>,
    pub revenue: Option<u64>,
    pub status: Option<String>,
    pub original_language: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    pub imdb_id: Option<String>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    #[serde(default)]
    pub production_companies: Vec<ProductionCompany>,
    #[serde(default)]
    pub production_countries: Vec<ProductionCountry>,
    #[serde(default)]
    pub spoken_languages: Vec<SpokenLanguage>,
    pub belongs_to_collection: Option<CollectionRef>,
    pub credits: Option<Credits>,
    pub videos: Option<VideoResults>,
    pub images: Option<ImageResults>,
    pub release_dates: Option<ReleaseDateResults>,
    pub keywords: Option<MovieKeywordResults>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Genre {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProductionCompany {
    pub id: u64,
    pub name: String,
    pub logo_path: Option<String>,
    pub origin_country: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProductionCountry {
    pub iso_3166_1: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SpokenLanguage {
    pub iso_639_1: String,
    pub name: String,
    pub english_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CollectionRef {
    pub id: u64,
    pub name: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Collection {
    pub id: u64,
    pub name: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    #[serde(default)]
    pub parts: Vec<CollectionPart>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CollectionPart {
    pub id: u64,
    pub title: Option<String>,
    pub release_date: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Credits {
    #[serde(default)]
    pub cast: Vec<CastMember>,
    #[serde(default)]
    pub crew: Vec<CrewMember>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CastMember {
    pub id: u64,
    pub name: String,
    pub character: Option<String>,
    pub order: Option<u32>,
    pub profile_path: Option<String>,
}

impl CastMember {
    pub fn profile(&self) -> String {
        format!("https://www.themoviedb.org/person/{}", self.id)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CrewMember {
    pub id: u64,
    pub name: String,
    pub job: String,
    pub department: Option<String>,
    pub profile_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VideoResults {
    #[serde(default)]
    pub results: Vec<Video>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Video {
    pub key: String,
    pub name: String,
    pub site: String,
    #[serde(rename = "type")]
    pub video_type: String,
    pub official: Option<bool>,
    pub iso_639_1: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImageResults {
    #[serde(default)]
    pub posters: Vec<Image>,
    #[serde(default)]
    pub backdrops: Vec<Image>,
    #[serde(default)]
    pub logos: Vec<Image>,
    #[serde(default)]
    pub stills: Vec<Image>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Image {
    pub file_path: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u32>,
    pub iso_639_1: Option<String>,
    pub aspect_ratio: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReleaseDateResults {
    #[serde(default)]
    pub results: Vec<ReleaseDateCountry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReleaseDateCountry {
    pub iso_3166_1: String,
    #[serde(default)]
    pub release_dates: Vec<ReleaseDate>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReleaseDate {
    pub certification: Option<String>,
    pub release_date: Option<String>,
    #[serde(rename = "type")]
    pub release_type: Option<u32>,
}

// TV Show types
#[derive(Debug, Deserialize, Serialize)]
pub struct TvShowDetail {
    pub id: u64,
    pub name: Option<String>,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub first_air_date: Option<String>,
    pub last_air_date: Option<String>,
    pub status: Option<String>,
    pub tagline: Option<String>,
    pub original_language: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u64>,
    #[serde(default)]
    pub episode_run_time: Vec<u32>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    #[serde(default)]
    pub production_companies: Vec<ProductionCompany>,
    #[serde(default)]
    pub origin_country: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub seasons: Vec<SeasonSummary>,
    #[serde(default)]
    pub networks: Vec<ProductionCompany>,
    #[serde(default)]
    pub created_by: Vec<CreatedBy>,
    pub number_of_seasons: Option<u32>,
    pub number_of_episodes: Option<u32>,
    pub credits: Option<Credits>,
    pub videos: Option<VideoResults>,
    pub images: Option<ImageResults>,
    pub content_ratings: Option<ContentRatings>,
    pub external_ids: Option<ExternalIds>,
    pub keywords: Option<KeywordResults>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SeasonSummary {
    pub id: u64,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub season_number: u32,
    pub episode_count: Option<u32>,
    pub air_date: Option<String>,
    pub poster_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatedBy {
    pub id: u64,
    pub name: String,
    pub profile_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContentRatings {
    #[serde(default)]
    pub results: Vec<ContentRating>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContentRating {
    pub iso_3166_1: String,
    pub rating: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExternalIds {
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<u64>,
}

// Keywords
#[derive(Debug, Deserialize, Serialize)]
pub struct KeywordResults {
    #[serde(default)]
    pub results: Vec<Keyword>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MovieKeywordResults {
    #[serde(default)]
    pub keywords: Vec<Keyword>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Keyword {
    pub id: u64,
    pub name: String,
}

// Season detail
#[derive(Debug, Deserialize, Serialize)]
pub struct SeasonDetail {
    pub id: u64,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub season_number: u32,
    pub air_date: Option<String>,
    pub poster_path: Option<String>,
    #[serde(default)]
    pub episodes: Vec<EpisodeSummary>,
    pub images: Option<ImageResults>,
    pub credits: Option<Credits>,
    pub videos: Option<VideoResults>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EpisodeSummary {
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
    pub crew: Option<Vec<CrewMember>>,
    pub guest_stars: Option<Vec<CastMember>>,
}
