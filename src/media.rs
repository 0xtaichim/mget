//! Local media discovery and Emby/Kodi file naming conventions.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

const MEDIA_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "wmv", "flv", "mov", "ts", "m2ts", "webm", "rmvb", "rm", "mpg", "mpeg",
    "iso", "strm",
];

/// Directories created by NAS software that never contain real episodes.
const IGNORED_DIRS: &[&str] = &["@eaDir", "#recycle", "$RECYCLE.BIN", "lost+found"];

const MAX_SCAN_DEPTH: usize = 8;

/// Local episode files keyed by `(season, episode)`. Several files may map to
/// the same episode (e.g. different editions or resolutions).
pub type EpisodeIndex = BTreeMap<(u32, u32), Vec<PathBuf>>;

pub fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| MEDIA_EXTENSIONS.iter().any(|m| m.eq_ignore_ascii_case(e)))
}

/// Parses the first `S<season>E<episode>` marker (case-insensitive) in a name.
pub fn parse_season_episode(name: &str) -> Option<(u32, u32)> {
    let bytes = name.as_bytes();
    (0..bytes.len()).find_map(|i| {
        if !bytes[i].eq_ignore_ascii_case(&b'S') {
            return None;
        }
        let (season, rest) = take_number(&bytes[i + 1..])?;
        let rest = rest
            .strip_prefix(b"E")
            .or_else(|| rest.strip_prefix(b"e"))?;
        let (episode, _) = take_number(rest)?;
        Some((season, episode))
    })
}

fn take_number(bytes: &[u8]) -> Option<(u32, &[u8])> {
    let len = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    let digits = std::str::from_utf8(&bytes[..len]).ok()?;
    Some((digits.parse().ok()?, &bytes[len..]))
}

/// Recursively scans a show directory for media files named with `SxxEyy`.
pub fn scan_tv_directory(dir: &Path) -> Result<EpisodeIndex> {
    if !dir.is_dir() {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Directory not found: {}", dir.display()),
        )));
    }
    let mut index = EpisodeIndex::new();
    scan_dir(dir, 0, &mut index)?;
    for paths in index.values_mut() {
        paths.sort();
    }
    Ok(index)
}

fn scan_dir(dir: &Path, depth: usize, index: &mut EpisodeIndex) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // Skips dotfiles, including macOS "._" AppleDouble companions of real videos.
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        // `fs::metadata` follows symlinks; the depth cap guards against cycles.
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            if depth < MAX_SCAN_DEPTH && !IGNORED_DIRS.contains(&name.as_ref()) {
                scan_dir(&path, depth + 1, index)?;
            }
        } else if is_media_file(&path)
            && let Some(key) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(parse_season_episode)
        {
            index.entry(key).or_default().push(path);
        }
    }
    Ok(())
}

/// `movie.mkv` → `movie.nfo`
pub fn nfo_path(media: &Path) -> PathBuf {
    media.with_extension("nfo")
}

/// `movie.mkv` + `-poster`, `jpg` → `movie-poster.jpg`
pub fn sidecar_path(media: &Path, suffix: &str, ext: &str) -> PathBuf {
    let stem = media.file_stem().unwrap_or_default().to_string_lossy();
    media.with_file_name(format!("{stem}{suffix}.{ext}"))
}

/// `episode.mkv` → `episode.jpg`
pub fn episode_thumb_path(media: &Path) -> PathBuf {
    media.with_extension("jpg")
}

/// Season poster in the show root: `season01-poster.jpg`, or
/// `season-specials-poster.jpg` for season 0.
pub fn season_poster_path(show_dir: &Path, season: u32) -> PathBuf {
    if season == 0 {
        show_dir.join("season-specials-poster.jpg")
    } else {
        show_dir.join(format!("season{season:02}-poster.jpg"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_season_episode_markers() {
        assert_eq!(parse_season_episode("Show.S01E02.1080p"), Some((1, 2)));
        assert_eq!(parse_season_episode("show s1e10"), Some((1, 10)));
        assert_eq!(parse_season_episode("剧名 S02E03 中字"), Some((2, 3)));
        assert_eq!(parse_season_episode("Sherlock S03E01"), Some((3, 1)));
        assert_eq!(parse_season_episode("Series Episode 1"), None);
        assert_eq!(parse_season_episode("S01"), None);
        assert_eq!(parse_season_episode("SE01"), None);
    }

    #[test]
    fn detects_media_files() {
        assert!(is_media_file(Path::new("a/b.MKV")));
        assert!(is_media_file(Path::new("b.strm")));
        assert!(!is_media_file(Path::new("b.nfo")));
        assert!(!is_media_file(Path::new("mkv")));
    }

    #[test]
    fn builds_sidecar_paths() {
        let media = Path::new("/m/Movie (2024).mkv");
        assert_eq!(nfo_path(media), Path::new("/m/Movie (2024).nfo"));
        assert_eq!(
            sidecar_path(media, "-clearlogo", "png"),
            Path::new("/m/Movie (2024)-clearlogo.png")
        );
        assert_eq!(
            season_poster_path(Path::new("/tv"), 3),
            Path::new("/tv/season03-poster.jpg")
        );
        assert_eq!(
            season_poster_path(Path::new("/tv"), 0),
            Path::new("/tv/season-specials-poster.jpg")
        );
    }

    #[test]
    fn scan_skips_hidden_and_system_entries() {
        let root = std::env::temp_dir().join(format!("mget-scan-{}", std::process::id()));
        let season = root.join("Season 1");
        fs::create_dir_all(&season).unwrap();
        fs::create_dir_all(root.join("@eaDir")).unwrap();
        for f in [
            "Season 1/Show S01E01.mkv",
            "Season 1/._Show S01E01.mkv",
            "Season 1/Show S01E01.2160p.mkv",
            "Season 1/Show S01E02.nfo",
            "@eaDir/Show S01E03.mkv",
            "Show S00E01.strm",
        ] {
            fs::write(root.join(f), b"").unwrap();
        }

        let index = scan_tv_directory(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(index.keys().copied().collect::<Vec<_>>(), [(0, 1), (1, 1)]);
        assert_eq!(index[&(1, 1)].len(), 2);
    }
}
