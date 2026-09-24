use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::config::ConfigKey;
use crate::tmdb::MediaType;

#[derive(Parser)]
#[command(name = "mget", version, about = "Media metadata scraper for Emby/Kodi")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Search for movies or TV shows on TMDB
    Search(SearchArgs),
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

#[derive(Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,
    /// Media type
    #[arg(short = 't', long = "type", value_enum, default_value_t = MediaType::Movie)]
    pub media_type: MediaType,
    /// Filter by release year (movies) or first air year (TV)
    #[arg(short, long)]
    pub year: Option<u32>,
    /// Language override (e.g. zh-CN, en)
    #[arg(short, long)]
    pub lang: Option<String>,
}

#[derive(Subcommand)]
pub enum FetchTarget {
    /// Fetch movie metadata and images
    Movie(MovieArgs),
    /// Fetch TV show metadata and images
    Tv(TvArgs),
}

#[derive(Args)]
pub struct MovieArgs {
    /// TMDB movie ID
    #[arg(long)]
    pub id: u64,
    /// Path to the source media file (.strm or video file)
    #[arg(short, long)]
    pub output: PathBuf,
    #[command(flatten)]
    pub options: FetchOptions,
}

#[derive(Args)]
pub struct TvArgs {
    /// TMDB TV show ID
    #[arg(long)]
    pub id: u64,
    /// Path to the TV show root directory
    #[arg(short, long)]
    pub output: PathBuf,
    /// Only fetch a specific season (skips show-level files)
    #[arg(long)]
    pub season: Option<u32>,
    #[command(flatten)]
    pub options: FetchOptions,
}

#[derive(Args)]
pub struct FetchOptions {
    /// Only generate NFO, skip image downloads
    #[arg(long, conflicts_with = "images_only")]
    pub no_images: bool,
    /// Only download images, skip NFO generation
    #[arg(long)]
    pub images_only: bool,
    /// Overwrite existing files
    #[arg(short, long)]
    pub force: bool,
    /// Also write a machine-readable result to stdout
    #[arg(long, value_enum, value_name = "FORMAT")]
    pub format: Option<OutputFormat>,
    /// Show what would be done without writing files
    #[arg(long)]
    pub dry_run: bool,
    /// Override image size (e.g. w500, w780, original)
    #[arg(long)]
    pub image_size: Option<String>,
    /// Language override
    #[arg(short, long)]
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Json,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set a configuration value (an empty value clears optional keys)
    Set {
        #[arg(value_enum)]
        key: ConfigKey,
        value: String,
    },
    /// Get a configuration value
    Get {
        #[arg(value_enum)]
        key: ConfigKey,
    },
    /// List all configuration values (secrets masked)
    List,
    /// Reset configuration to defaults
    Reset,
    /// Print the configuration file path
    Path,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn image_flags_conflict() {
        let args = ["mget", "fetch", "movie", "--id", "1", "-o", "x.mkv"];
        assert!(Cli::try_parse_from(args).is_ok());
        let conflicting = [&args[..], &["--no-images", "--images-only"]].concat();
        assert!(Cli::try_parse_from(conflicting).is_err());
    }
}
