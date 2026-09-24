mod config;
mod download;
mod error;
mod media;
mod nfo;
mod tmdb;

use clap::{Parser, Subcommand};
use serde::Serialize;
use std::path::PathBuf;
use std::process;

use crate::config::Config;
use crate::download::{DownloadTask, Downloader};
use crate::error::Error;
use crate::tmdb::TmdbClient;

#[derive(Parser)]
#[command(name = "mget", version, about = "Media metadata scraper for Emby/Kodi")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for movies or TV shows on TMDB
    Search {
        /// Search query
        query: String,
        /// Media type: movie or tv
        #[arg(short = 't', long = "type")]
        media_type: Option<String>,
        /// Filter by year
        #[arg(short, long)]
        year: Option<u32>,
        /// Language override (e.g. zh-CN, en)
        #[arg(short, long)]
        lang: Option<String>,
    },
    /// Fetch metadata and images for a movie or TV show
    Fetch {
        #[command(subcommand)]
        target: FetchTarget,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum FetchTarget {
    /// Fetch movie metadata and images
    Movie {
        /// TMDB movie ID
        #[arg(long)]
        id: u64,
        /// Path to the source media file (.strm or video file)
        #[arg(short, long)]
        output: PathBuf,
        /// Only generate NFO, skip image downloads
        #[arg(long)]
        no_images: bool,
        /// Only download images, skip NFO generation
        #[arg(long)]
        images_only: bool,
        /// Overwrite existing files
        #[arg(short, long)]
        force: bool,
        /// Also output JSON result to stdout
        #[arg(long, value_name = "FORMAT")]
        format: Option<String>,
        /// Show what would be done without writing files
        #[arg(long)]
        dry_run: bool,
        /// Override image size (e.g. w500, w780, original)
        #[arg(long)]
        image_size: Option<String>,
        /// Language override
        #[arg(short, long)]
        lang: Option<String>,
    },
    /// Fetch TV show metadata and images
    Tv {
        /// TMDB TV show ID
        #[arg(long)]
        id: u64,
        /// Path to the TV show root directory
        #[arg(short, long)]
        output: PathBuf,
        /// Only fetch a specific season
        #[arg(long)]
        season: Option<u32>,
        /// Only generate NFO, skip image downloads
        #[arg(long)]
        no_images: bool,
        /// Only download images, skip NFO generation
        #[arg(long)]
        images_only: bool,
        /// Overwrite existing files
        #[arg(short, long)]
        force: bool,
        /// Also output JSON result to stdout
        #[arg(long, value_name = "FORMAT")]
        format: Option<String>,
        /// Show what would be done without writing files
        #[arg(long)]
        dry_run: bool,
        /// Override image size (e.g. w500, w780, original)
        #[arg(long)]
        image_size: Option<String>,
        /// Language override
        #[arg(short, long)]
        lang: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set a configuration value
    Set { key: String, value: String },
    /// Get a configuration value
    Get { key: String },
    /// List all configuration values
    List,
    /// Reset configuration to defaults
    Reset,
}

// JSON output types for structured stdout
#[derive(Serialize)]
struct SearchOutput {
    results: Vec<SearchResultOutput>,
    total_results: u32,
}

#[derive(Serialize)]
struct SearchResultOutput {
    id: u64,
    title: Option<String>,
    original_title: Option<String>,
    year: Option<String>,
    overview: Option<String>,
    poster_url: Option<String>,
    vote_average: Option<f64>,
}

#[derive(Serialize)]
struct FetchOutput {
    id: u64,
    title: Option<String>,
    files_written: Vec<String>,
    files_skipped: Vec<String>,
    errors: Vec<FetchError>,
}

#[derive(Serialize)]
struct FetchError {
    file: String,
    error: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let exit_code = match run(cli).await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {}", e);
            e.exit_code()
        }
    };
    process::exit(exit_code);
}

async fn run(cli: Cli) -> error::Result<()> {
    match cli.command {
        Commands::Search {
            query,
            media_type,
            year,
            lang,
        } => cmd_search(&query, media_type.as_deref(), year, lang.as_deref()).await,
        Commands::Fetch { target } => cmd_fetch(target).await,
        Commands::Config { action } => cmd_config(action),
    }
}

async fn cmd_search(
    query: &str,
    media_type: Option<&str>,
    year: Option<u32>,
    lang: Option<&str>,
) -> error::Result<()> {
    let config = Config::load()?;
    let client = TmdbClient::new(&config)?;
    let mt = media_type.unwrap_or("movie");
    let results = client.search(query, mt, year, lang).await?;

    if results.results.is_empty() {
        return Err(Error::NotFound(format!(
            "No results found for '{}'",
            query
        )));
    }

    let image_base = "https://image.tmdb.org/t/p/";
    let output = SearchOutput {
        total_results: results.total_results,
        results: results
            .results
            .into_iter()
            .map(|r| {
                let year = r
                    .release_date
                    .as_deref()
                    .filter(|d| d.len() >= 4)
                    .map(|d| d[..4].to_string());
                let poster_url = r
                    .poster_path
                    .as_ref()
                    .map(|p| format!("{}w500{}", image_base, p));
                SearchResultOutput {
                    id: r.id,
                    title: r.title,
                    original_title: r.original_title,
                    year,
                    overview: r.overview,
                    poster_url,
                    vote_average: r.vote_average,
                }
            })
            .collect(),
    };

    println!("{}", serde_json::to_string_pretty(&output)
        .map_err(|e| Error::Config(format!("Failed to serialize JSON: {}", e)))?);
    Ok(())
}

async fn cmd_fetch(target: FetchTarget) -> error::Result<()> {
    match target {
        FetchTarget::Movie {
            id,
            output,
            no_images,
            images_only,
            force,
            format,
            dry_run,
            image_size,
            lang,
        } => {
            cmd_fetch_movie(id, &output, no_images, images_only, force, format, dry_run, image_size, lang).await
        }
        FetchTarget::Tv {
            id,
            output,
            season,
            no_images,
            images_only,
            force,
            format,
            dry_run,
            image_size,
            lang,
        } => {
            cmd_fetch_tv(id, &output, season, no_images, images_only, force, format, dry_run, image_size, lang).await
        }
    }
}

async fn cmd_fetch_movie(
    id: u64,
    output: &PathBuf,
    no_images: bool,
    images_only: bool,
    force: bool,
    format: Option<String>,
    dry_run: bool,
    image_size: Option<String>,
    lang: Option<String>,
) -> error::Result<()> {
    let mut config = Config::load()?;
    if let Some(size) = &image_size {
        config.defaults.image_size = size.clone();
    }

    let client = TmdbClient::new(&config)?;
    let movie = client.get_movie_fallback(id, lang.as_deref()).await?;

    let collection = if let Some(coll_ref) = &movie.belongs_to_collection {
        client.get_collection(coll_ref.id).await.ok()
    } else {
        None
    };

    let image_base = "https://image.tmdb.org/t/p/";
    let overwrite = force || config.defaults.overwrite;

    let mut files_written = Vec::new();
    let mut files_skipped = Vec::new();
    let mut errors = Vec::new();

    // Generate NFO
    if !images_only {
        let nfo_file = media::nfo_path(output);
        if dry_run {
            eprintln!("[dry-run] Would write: {}", nfo_file.display());
        } else if !overwrite && nfo_file.exists() {
            files_skipped.push(nfo_file.display().to_string());
            eprintln!("Skipped (exists): {}", nfo_file.display());
        } else {
            let nfo_content =
                nfo::generate_movie_nfo(&movie, collection.as_ref(), image_base)?;
            std::fs::write(&nfo_file, &nfo_content)?;
            files_written.push(nfo_file.display().to_string());
            eprintln!("Written: {}", nfo_file.display());
        }
    }

    // Download images
    if !no_images {
        let mut tasks = Vec::new();
        let size = config.defaults.image_size.as_str();

        // Select best images from the images response
        if let Some(images) = &movie.images {
            if let Some(poster) = select_best_image(&images.posters, &config) {
                tasks.push(DownloadTask {
                    url: format!("{}{}{}", image_base, size, poster),
                    path: media::movie_image_path(output, "-poster", "jpg"),
                    description: "poster".into(),
                });
            }
            if let Some(backdrop) = select_best_image(&images.backdrops, &config) {
                tasks.push(DownloadTask {
                    url: format!("{}{}{}", image_base, size, backdrop),
                    path: media::movie_image_path(output, "-fanart", "jpg"),
                    description: "fanart".into(),
                });
            }
            if let Some(logo) = select_best_image(&images.logos, &config) {
                tasks.push(DownloadTask {
                    url: format!("{}{}{}", image_base, size, logo),
                    path: media::movie_image_path(output, "-clearlogo", "png"),
                    description: "clearlogo".into(),
                });
            }
        } else {
            // Fallback to poster_path / backdrop_path from main response
            if let Some(poster) = &movie.poster_path {
                tasks.push(DownloadTask {
                    url: format!("{}{}{}", image_base, size, poster),
                    path: media::movie_image_path(output, "-poster", "jpg"),
                    description: "poster".into(),
                });
            }
            if let Some(backdrop) = &movie.backdrop_path {
                tasks.push(DownloadTask {
                    url: format!("{}{}{}", image_base, size, backdrop),
                    path: media::movie_image_path(output, "-fanart", "jpg"),
                    description: "fanart".into(),
                });
            }
        }

        if dry_run {
            for task in &tasks {
                eprintln!("[dry-run] Would download {} -> {}", task.description, task.path.display());
            }
        } else {
            let downloader = Downloader::new(&config)?;
            let results = downloader.download_all(tasks, overwrite).await;
            for r in results {
                if r.success {
                    if r.error.as_deref() == Some("skipped (already exists)") {
                        files_skipped.push(r.path.display().to_string());
                        eprintln!("Skipped (exists): {}", r.path.display());
                    } else {
                        files_written.push(r.path.display().to_string());
                        eprintln!("Downloaded: {} -> {}", r.description, r.path.display());
                    }
                } else {
                    let err_msg = r.error.unwrap_or_default();
                    errors.push(FetchError {
                        file: r.path.display().to_string(),
                        error: err_msg.clone(),
                    });
                    eprintln!("Failed: {} - {}", r.description, err_msg);
                }
            }
        }
    }

    // Summary to stderr
    eprintln!(
        "\nDone: {} written, {} skipped, {} errors",
        files_written.len(),
        files_skipped.len(),
        errors.len()
    );

    // JSON output to stdout if requested
    if format.as_deref() == Some("json") {
        let output = FetchOutput {
            id: movie.id,
            title: movie.title,
            files_written,
            files_skipped,
            errors,
        };
        println!("{}", serde_json::to_string_pretty(&output)
            .map_err(|e| Error::Config(format!("Failed to serialize JSON: {}", e)))?);
    }

    Ok(())
}

async fn cmd_fetch_tv(
    id: u64,
    output: &PathBuf,
    season_filter: Option<u32>,
    no_images: bool,
    images_only: bool,
    force: bool,
    format: Option<String>,
    dry_run: bool,
    image_size: Option<String>,
    lang: Option<String>,
) -> error::Result<()> {
    let mut config = Config::load()?;
    if let Some(size) = &image_size {
        config.defaults.image_size = size.clone();
    }

    let client = TmdbClient::new(&config)?;
    let show = client.get_tv_show_fallback(id, lang.as_deref()).await?;

    let image_base = "https://image.tmdb.org/t/p/";
    let overwrite = force || config.defaults.overwrite;

    let mut files_written = Vec::new();
    let mut files_skipped = Vec::new();
    let mut errors = Vec::new();

    // Create downloader once for reuse across all episodes and seasons
    let downloader = Downloader::new(&config)?;

    // Scan for episode files in the directory
    let episode_files = media::scan_tv_directory(output)?;

    // Determine which seasons to process
    let seasons_to_fetch: Vec<u32> = if let Some(s) = season_filter {
        vec![s]
    } else {
        show.seasons
            .iter()
            .filter(|s| s.season_number > 0)
            .map(|s| s.season_number)
            .collect()
    };

    // === Show-level files ===
    if season_filter.is_none() {
        // tvshow.nfo
        if !images_only {
            let nfo_file = output.join("tvshow.nfo");
            write_nfo_file(
                &nfo_file,
                || nfo::generate_tvshow_nfo(&show, &config.tmdb.api_key, image_base),
                overwrite,
                dry_run,
                &mut files_written,
                &mut files_skipped,
            )?;
        }

        // Show-level images
        if !no_images && !dry_run {
            let mut tasks = Vec::new();
            let size = config.defaults.image_size.as_str();

            if let Some(images) = &show.images {
                if let Some(poster) = select_best_image(&images.posters, &config) {
                    tasks.push(DownloadTask {
                        url: format!("{}{}{}", image_base, size, poster),
                        path: media::show_image_path(output, "poster.jpg"),
                        description: "show poster".into(),
                    });
                }
                if let Some(backdrop) = select_best_image(&images.backdrops, &config) {
                    tasks.push(DownloadTask {
                        url: format!("{}{}{}", image_base, size, backdrop),
                        path: media::show_image_path(output, "fanart.jpg"),
                        description: "show fanart".into(),
                    });
                }
                if let Some(logo) = select_best_image(&images.logos, &config) {
                    tasks.push(DownloadTask {
                        url: format!("{}{}{}", image_base, size, logo),
                        path: media::show_image_path(output, "clearlogo.png"),
                        description: "show clearlogo".into(),
                    });
                }
            } else {
                if let Some(poster) = &show.poster_path {
                    tasks.push(DownloadTask {
                        url: format!("{}{}{}", image_base, size, poster),
                        path: media::show_image_path(output, "poster.jpg"),
                        description: "show poster".into(),
                    });
                }
                if let Some(backdrop) = &show.backdrop_path {
                    tasks.push(DownloadTask {
                        url: format!("{}{}{}", image_base, size, backdrop),
                        path: media::show_image_path(output, "fanart.jpg"),
                        description: "show fanart".into(),
                    });
                }
            }

            if !tasks.is_empty() {
                collect_download_results(
                    downloader.download_all(tasks, overwrite).await,
                    &mut files_written,
                    &mut files_skipped,
                    &mut errors,
                );
            }
        } else if !no_images && dry_run {
            eprintln!("[dry-run] Would download show-level images to {}", output.display());
        }
    }

    // === Season-level processing ===
    for season_num in &seasons_to_fetch {
        eprintln!("Processing Season {}...", season_num);

        let season = match client.get_tv_season(id, *season_num, lang.as_deref()).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Warning: Failed to fetch Season {}: {}", season_num, e);
                errors.push(FetchError {
                    file: format!("Season {}", season_num),
                    error: e.to_string(),
                });
                continue;
            }
        };

        // Fetch English season for original episode titles
        let en_episode_names: std::collections::HashMap<u32, String> =
            if lang.as_deref().unwrap_or(&config.defaults.language) != "en" {
                client
                    .get_tv_season(id, *season_num, Some("en"))
                    .await
                    .map(|en_season| {
                        en_season
                            .episodes
                            .into_iter()
                            .filter_map(|ep| ep.name.map(|n| (ep.episode_number, n)))
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                std::collections::HashMap::new()
            };

        // Season poster
        if !no_images {
            let size = config.defaults.image_size.as_str();
            let mut tasks = Vec::new();

            // Prefer season.poster_path (TMDB's default, usually best quality)
            let season_poster = season
                .poster_path
                .as_deref()
                .or_else(|| {
                    season
                        .images
                        .as_ref()
                        .and_then(|imgs| select_best_image(&imgs.posters, &config))
                })
                .map(|p| p.to_string());

            if let Some(poster) = season_poster {
                tasks.push(DownloadTask {
                    url: format!("{}{}{}", image_base, size, poster),
                    path: media::season_image_path(output, *season_num, "-poster"),
                    description: format!("season {} poster", season_num),
                });
            }

            if dry_run {
                for task in &tasks {
                    eprintln!("[dry-run] Would download {} -> {}", task.description, task.path.display());
                }
            } else if !tasks.is_empty() {
                collect_download_results(
                    downloader.download_all(tasks, overwrite).await,
                    &mut files_written,
                    &mut files_skipped,
                    &mut errors,
                );
            }
        }

        // === Episode-level processing ===
        for ep_data in &season.episodes {
            // Find matching local file
            let matching_file = episode_files.iter().find(|f| {
                f.season == ep_data.season_number && f.episode == ep_data.episode_number
            });

            let ep_file = match matching_file {
                Some(f) => f,
                None => {
                    eprintln!(
                        "No local file for S{:02}E{:02}, skipping",
                        ep_data.season_number, ep_data.episode_number
                    );
                    continue;
                }
            };

            // Episode NFO
            if !images_only {
                let nfo_file = media::nfo_path(&ep_file.path);
                let ep = ep_data;
                let show_ref = &show;
                let orig_title = en_episode_names.get(&ep_data.episode_number).map(|s| s.as_str());
                write_nfo_file(
                    &nfo_file,
                    || nfo::generate_episode_nfo(ep, show_ref, orig_title, image_base),
                    overwrite,
                    dry_run,
                    &mut files_written,
                    &mut files_skipped,
                )?;
            }

            // Episode thumbnail
            if !no_images {
                if let Some(still) = &ep_data.still_path {
                    let size = config.defaults.image_size.as_str();
                    let task = DownloadTask {
                        url: format!("{}{}{}", image_base, size, still),
                        path: media::episode_image_path(&ep_file.path),
                        description: format!(
                            "S{:02}E{:02} thumb",
                            ep_data.season_number, ep_data.episode_number
                        ),
                    };

                    if dry_run {
                        eprintln!(
                            "[dry-run] Would download {} -> {}",
                            task.description,
                            task.path.display()
                        );
                    } else {
                        collect_download_results(
                            downloader.download_all(vec![task], overwrite).await,
                            &mut files_written,
                            &mut files_skipped,
                            &mut errors,
                        );
                    }
                }
            }
        }
    }

    // Summary
    eprintln!(
        "\nDone: {} written, {} skipped, {} errors",
        files_written.len(),
        files_skipped.len(),
        errors.len()
    );

    if format.as_deref() == Some("json") {
        let out = FetchOutput {
            id: show.id,
            title: show.name,
            files_written,
            files_skipped,
            errors,
        };
        println!("{}", serde_json::to_string_pretty(&out)
            .map_err(|e| Error::Config(format!("Failed to serialize JSON: {}", e)))?);
    }

    Ok(())
}

fn write_nfo_file(
    path: &PathBuf,
    generate: impl FnOnce() -> error::Result<String>,
    overwrite: bool,
    dry_run: bool,
    written: &mut Vec<String>,
    skipped: &mut Vec<String>,
) -> error::Result<()> {
    if dry_run {
        eprintln!("[dry-run] Would write: {}", path.display());
        return Ok(());
    }
    if !overwrite && path.exists() {
        skipped.push(path.display().to_string());
        eprintln!("Skipped (exists): {}", path.display());
        return Ok(());
    }
    let content = generate()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, &content)?;
    written.push(path.display().to_string());
    eprintln!("Written: {}", path.display());
    Ok(())
}

fn collect_download_results(
    results: Vec<download::DownloadResult>,
    written: &mut Vec<String>,
    skipped: &mut Vec<String>,
    errors: &mut Vec<FetchError>,
) {
    for r in results {
        if r.success {
            if r.error.as_deref() == Some("skipped (already exists)") {
                skipped.push(r.path.display().to_string());
                eprintln!("Skipped (exists): {}", r.path.display());
            } else {
                written.push(r.path.display().to_string());
                eprintln!("Downloaded: {} -> {}", r.description, r.path.display());
            }
        } else {
            let err_msg = r.error.unwrap_or_default();
            errors.push(FetchError {
                file: r.path.display().to_string(),
                error: err_msg.clone(),
            });
            eprintln!("Failed: {} - {}", r.description, err_msg);
        }
    }
}

/// Compare images by vote_average first, then by resolution (width) as tiebreaker.
fn image_quality_cmp(a: &&tmdb::types::Image, b: &&tmdb::types::Image) -> std::cmp::Ordering {
    a.vote_average
        .partial_cmp(&b.vote_average)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| a.width.unwrap_or(0).cmp(&b.width.unwrap_or(0)))
}

/// Select the best image from a list based on language preferences and quality.
fn select_best_image<'a>(
    images: &'a [tmdb::types::Image],
    config: &Config,
) -> Option<&'a str> {
    if images.is_empty() {
        return None;
    }

    let lang_prefs: Vec<String> = config
        .defaults
        .image_language
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if !lang_prefs.is_empty() {
        for pref in &lang_prefs {
            let best = images
                .iter()
                .filter(|img| {
                    img.iso_639_1
                        .as_deref()
                        .map(|l| l == pref)
                        .unwrap_or(false)
                })
                .max_by(image_quality_cmp);
            if let Some(img) = best {
                return Some(&img.file_path);
            }
        }
        let no_lang = images
            .iter()
            .filter(|img| img.iso_639_1.is_none() || img.iso_639_1.as_deref() == Some(""))
            .max_by(image_quality_cmp);
        if let Some(img) = no_lang {
            return Some(&img.file_path);
        }
    }

    images
        .iter()
        .max_by(image_quality_cmp)
        .map(|img| img.file_path.as_str())
}

fn cmd_config(action: ConfigAction) -> error::Result<()> {
    match action {
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;
            config.set(&key, &value)?;
            config.save()?;
            eprintln!("Set {} = {}", key, value);
            Ok(())
        }
        ConfigAction::Get { key } => {
            let config = Config::load()?;
            let value = config.get(&key)?;
            println!("{}", value);
            Ok(())
        }
        ConfigAction::List => {
            let config = Config::load()?;
            let items = config.list();
            let output: serde_json::Value = items
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect::<serde_json::Map<String, serde_json::Value>>()
                .into();
            println!("{}", serde_json::to_string_pretty(&output)
                .map_err(|e| Error::Config(format!("Failed to serialize JSON: {}", e)))?);
            Ok(())
        }
        ConfigAction::Reset => {
            let config = Config::default();
            config.save()?;
            eprintln!("Configuration reset to defaults");
            Ok(())
        }
    }
}
