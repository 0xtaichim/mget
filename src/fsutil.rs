use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Writes `contents` to `path` via a sibling temp file and rename, so readers
/// (and interrupted runs) never observe a partially written file.
pub fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent {
        fs::create_dir_all(parent)?;
    }
    let tmp = temp_path(path);
    let result = fs::write(&tmp, contents).and_then(|()| fs::rename(&tmp, path));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.mget-tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_creates_parents() {
        let dir = std::env::temp_dir().join(format!("mget-fsutil-{}", std::process::id()));
        let target = dir.join("a/b/file.txt");
        write_atomic(&target, b"hello").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"hello");
        write_atomic(&target, b"world").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"world");
        assert!(!temp_path(&target).exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
