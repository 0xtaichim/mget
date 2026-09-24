use std::collections::HashMap;
use std::path::{Path, PathBuf};

use futures_util::{StreamExt, stream};
use reqwest::Client;
use serde::Serialize;

use crate::cli::{FetchOptions, MovieArgs, OutputFormat, TvArgs};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::plan::{ExecOptions, Plan, Report};
use crate::tmdb::types::{Image, ImageResults, SeasonDetail, TvShowDetail};
use crate::tmdb::{self, TmdbClient, images};
use crate::{http, media, nfo};

/// Language whose episode titles fill `<originaltitle>` in episode NFOs.
const ORIGINAL_TITLE_LANGUAGE: &str = "en";

#[derive(Serialize)]
struct FetchOutput {
    id: u64,
    title: Option<String>,
    #[serde(flatten)]
    report: Report,
}

/// Everything a fetch needs, resolved once from config and CLI options.
struct Fetcher {
    http: Client,
    tmdb: TmdbClient,
    language: String,
    image_size: String,
    image_languages: Vec<String>,
    write_nfo: bool,
    write_images: bool,
    format: Option<OutputFormat>,
    exec: ExecOptions,
}

impl Fetcher {
    fn new(opts: FetchOptions) -> Result<Self> {
        let config = Config::load()?;
        let http = http::build_client(&config.network)?;
        let tmdb = TmdbClient::new(http.clone(), &config)?;
        Ok(Self {
            http,
            tmdb,
            language: opts
                .lang
                .unwrap_or_else(|| config.defaults.language.clone()),
            image_size: opts
                .image_size
                .unwrap_or_else(|| config.defaults.image_size.clone()),
            image_languages: config.image_languages(),
            write_nfo: !opts.images_only,
            write_images: !opts.no_images,
            format: opts.format,
            exec: ExecOptions {
                overwrite: opts.force || config.defaults.overwrite,
                dry_run: opts.dry_run,
                concurrency: config.network.concurrent_downloads.max(1) as usize,
            },
        })
    }

    fn image_url(&self, path: &str) -> String {
        images::url(&self.image_size, path)
    }

    /// Adds poster / fanart / clearlogo downloads, preferring the best-rated
    /// image in the preferred language over the TMDB default.
    fn plan_artwork(
        &self,
        plan: &mut Plan,
        available: Option<&ImageResults>,
        default_poster: Option<&str>,
        default_backdrop: Option<&str>,
        target: impl Fn(Artwork) -> (PathBuf, String),
    ) {
        let langs = &self.image_languages;
        let best = |pick: fn(&ImageResults) -> &[Image]| {
            available
                .and_then(|imgs| images::select_best(pick(imgs), langs))
                .map(|img| img.file_path.as_str())
        };
        let logo = available
            .and_then(|imgs| {
                images::select_best(imgs.logos.iter().filter(|i| images::is_raster(i)), langs)
            })
            .map(|img| img.file_path.as_str());

        for (kind, source) in [
            (Artwork::Poster, best(|i| &i.posters).or(default_poster)),
            (Artwork::Fanart, best(|i| &i.backdrops).or(default_backdrop)),
            (Artwork::ClearLogo, logo),
        ] {
            if let Some(source) = source {
                let (path, label) = target(kind);
                plan.image(path, self.image_url(source), label);
            }
        }
    }

    /// Fetches a season and, concurrently, its episode titles in the original-title language.
    async fn season(&self, tv_id: u64, number: u32) -> (u32, Result<Season>) {
        let original_titles = async {
            if tmdb::primary_subtag(&self.language) == ORIGINAL_TITLE_LANGUAGE {
                return HashMap::new();
            }
            self.tmdb
                .episode_titles(tv_id, number, ORIGINAL_TITLE_LANGUAGE)
                .await
                .unwrap_or_default()
        };
        let (detail, original_titles) = tokio::join!(
            self.tmdb.season(tv_id, number, &self.language),
            original_titles
        );
        let season = detail.map(|detail| Season {
            number,
            detail,
            original_titles,
        });
        (number, season)
    }

    fn plan_season(&self, plan: &mut Plan, target: &ShowTarget, season: &Season) {
        let number = season.number;
        if self.write_images {
            // TMDB's default season poster is usually the best curated one.
            let poster = season.detail.poster_path.as_deref().or_else(|| {
                season
                    .detail
                    .images
                    .as_ref()
                    .and_then(|imgs| images::select_best(&imgs.posters, &self.image_languages))
                    .map(|img| img.file_path.as_str())
            });
            if let Some(poster) = poster {
                plan.image(
                    media::season_poster_path(target.dir, number),
                    self.image_url(poster),
                    format!("season {number} poster"),
                );
            }
        }

        let mut missing = Vec::new();
        for ep in &season.detail.episodes {
            let Some(files) = target.local.get(&(ep.season_number, ep.episode_number)) else {
                missing.push(format!("E{:02}", ep.episode_number));
                continue;
            };
            let content = self.write_nfo.then(|| {
                let original = season.original_titles.get(&ep.episode_number);
                nfo::episode(ep, target.show, original.map(String::as_str))
            });
            for file in files {
                if let Some(content) = &content {
                    plan.nfo(media::nfo_path(file), content.clone());
                }
                if self.write_images
                    && let Some(still) = &ep.still_path
                {
                    plan.image(
                        media::episode_thumb_path(file),
                        self.image_url(still),
                        format!("S{:02}E{:02} thumb", ep.season_number, ep.episode_number),
                    );
                }
            }
        }
        if !missing.is_empty() {
            eprintln!(
                "Season {number}: no local file for {}, skipping",
                missing.join(", ")
            );
        }
    }

    async fn finish(&self, plan: Plan, id: u64, title: Option<String>) -> Result<()> {
        let report = plan.execute(&self.http, &self.exec).await;
        report.print_summary();
        let failures = report.errors.len();
        if self.format == Some(OutputFormat::Json) {
            super::print_json(&FetchOutput { id, title, report })?;
        }
        match failures {
            0 => Ok(()),
            n => Err(Error::Incomplete(n)),
        }
    }
}

/// The local show directory being populated, and what TMDB says about it.
struct ShowTarget<'a> {
    dir: &'a Path,
    local: &'a media::EpisodeIndex,
    show: &'a TvShowDetail,
}

struct Season {
    number: u32,
    detail: SeasonDetail,
    /// Episode number → title in [`ORIGINAL_TITLE_LANGUAGE`].
    original_titles: HashMap<u32, String>,
}

#[derive(Clone, Copy)]
enum Artwork {
    Poster,
    Fanart,
    ClearLogo,
}

impl Artwork {
    fn name(self) -> &'static str {
        match self {
            Artwork::Poster => "poster",
            Artwork::Fanart => "fanart",
            Artwork::ClearLogo => "clearlogo",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Artwork::ClearLogo => "png",
            Artwork::Poster | Artwork::Fanart => "jpg",
        }
    }
}

pub async fn movie(args: MovieArgs) -> Result<()> {
    let fetcher = Fetcher::new(args.options)?;
    let movie = fetcher.tmdb.movie(args.id, &fetcher.language).await?;
    let output = args.output.as_path();
    let mut plan = Plan::default();

    if fetcher.write_nfo {
        let collection = match &movie.belongs_to_collection {
            Some(c) => fetcher.tmdb.collection(c.id, &fetcher.language).await.ok(),
            None => None,
        };
        plan.nfo(
            media::nfo_path(output),
            nfo::movie(&movie, collection.as_ref()),
        );
    }

    if fetcher.write_images {
        fetcher.plan_artwork(
            &mut plan,
            movie.images.as_ref(),
            movie.poster_path.as_deref(),
            movie.backdrop_path.as_deref(),
            |art| {
                let suffix = format!("-{}", art.name());
                (
                    media::sidecar_path(output, &suffix, art.extension()),
                    art.name().into(),
                )
            },
        );
    }

    fetcher.finish(plan, movie.id, movie.title).await
}

pub async fn tv(args: TvArgs) -> Result<()> {
    let fetcher = Fetcher::new(args.options)?;
    let output = args.output.as_path();
    let local = media::scan_tv_directory(output)?;
    let show = fetcher.tmdb.tv_show(args.id, &fetcher.language).await?;
    let mut plan = Plan::default();

    if args.season.is_none() {
        if fetcher.write_nfo {
            plan.nfo(output.join("tvshow.nfo"), nfo::tvshow(&show));
        }
        if fetcher.write_images {
            fetcher.plan_artwork(
                &mut plan,
                show.images.as_ref(),
                show.poster_path.as_deref(),
                show.backdrop_path.as_deref(),
                |art| {
                    let file = format!("{}.{}", art.name(), art.extension());
                    (output.join(file), format!("show {}", art.name()))
                },
            );
        }
    }

    // Specials (season 0) are only worth fetching when there are local files for them.
    let has_specials = local.keys().any(|&(season, _)| season == 0);
    let season_numbers: Vec<u32> = match args.season {
        Some(n) => vec![n],
        None => show
            .seasons
            .iter()
            .map(|s| s.season_number)
            .filter(|&n| n > 0 || has_specials)
            .collect(),
    };

    let seasons: Vec<_> = stream::iter(season_numbers)
        .map(|n| fetcher.season(show.id, n))
        .buffered(fetcher.exec.concurrency)
        .collect()
        .await;

    let target = ShowTarget {
        dir: output,
        local: &local,
        show: &show,
    };
    for (number, result) in seasons {
        eprintln!("Processing Season {number}...");
        match result {
            Ok(season) => fetcher.plan_season(&mut plan, &target, &season),
            Err(e) => {
                eprintln!("Warning: Failed to fetch Season {number}: {e}");
                plan.fail(format!("Season {number}"), e);
            }
        }
    }

    fetcher.finish(plan, show.id, show.name).await
}
