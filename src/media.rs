use std::path::{Path, PathBuf};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct EpisodeFile {
    pub path: PathBuf,
    pub season: u32,
    pub episode: u32,
}

const MEDIA_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "wmv", "flv", "mov", "ts", "m2ts",
    "webm", "rmvb", "rm", "mpg", "mpeg", "iso", "strm",
];

pub fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| MEDIA_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// Parse season and episode numbers from a filename.
/// Supports patterns like S01E01, S1E1, s01e01, etc.
pub fn parse_season_episode(filename: &str) -> Option<(u32, u32)> {
    // Try S01E01 pattern (case insensitive)
    let upper = filename.to_uppercase();

    // Look for SxxExx pattern
    let mut i = 0;
    let chars: Vec<char> = upper.chars().collect();
    while i < chars.len() {
        if chars[i] == 'S' {
            let s_start = i + 1;
            let mut s_end = s_start;
            while s_end < chars.len() && chars[s_end].is_ascii_digit() {
                s_end += 1;
            }
            if s_end > s_start && s_end < chars.len() && chars[s_end] == 'E' {
                let e_start = s_end + 1;
                let mut e_end = e_start;
                while e_end < chars.len() && chars[e_end].is_ascii_digit() {
                    e_end += 1;
                }
                if e_end > e_start {
                    let season: String = chars[s_start..s_end].iter().collect();
                    let episode: String = chars[e_start..e_end].iter().collect();
                    if let (Ok(s), Ok(e)) = (season.parse::<u32>(), episode.parse::<u32>()) {
                        return Some((s, e));
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Scan a TV show directory for media files, extracting season/episode info.
/// Looks in subdirectories (Season X, S01, etc.) and the root directory.
pub fn scan_tv_directory(dir: &Path) -> Result<Vec<EpisodeFile>> {
    let mut episodes = Vec::new();

    if !dir.is_dir() {
        return Err(Error::FileSystem(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Directory not found: {}", dir.display()),
        )));
    }

    // Scan the root and all subdirectories
    scan_dir_recursive(dir, &mut episodes)?;

    episodes.sort_by(|a, b| {
        a.season.cmp(&b.season).then(a.episode.cmp(&b.episode))
    });

    Ok(episodes)
}

fn scan_dir_recursive(dir: &Path, episodes: &mut Vec<EpisodeFile>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan_dir_recursive(&path, episodes)?;
        } else if is_media_file(&path) {
            let stem = file_stem(&path);
            if let Some((season, episode)) = parse_season_episode(&stem) {
                episodes.push(EpisodeFile {
                    path,
                    season,
                    episode,
                });
            }
        }
    }
    Ok(())
}

/// Get the NFO path for a media file (same name, .nfo extension).
pub fn nfo_path(media_path: &Path) -> PathBuf {
    media_path.with_extension("nfo")
}

/// Get the image path for a movie file with a suffix like "-poster", "-fanart", etc.
pub fn movie_image_path(media_path: &Path, suffix: &str, ext: &str) -> PathBuf {
    let stem = file_stem(media_path);
    let parent = media_path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{}{}.{}", stem, suffix, ext))
}

/// Get the episode thumbnail path (same name, .jpg extension for episodes).
pub fn episode_image_path(media_path: &Path) -> PathBuf {
    media_path.with_extension("jpg")
}

/// Get show-level image path in the root directory.
pub fn show_image_path(show_dir: &Path, name: &str) -> PathBuf {
    show_dir.join(name)
}

/// Get season poster path like "season01-poster.jpg" in the show root directory.
pub fn season_image_path(show_dir: &Path, season_number: u32, suffix: &str) -> PathBuf {
    show_dir.join(format!("season{:02}{}.jpg", season_number, suffix))
}
