use crate::linkleaf_proto::Feed;
use anyhow::{Context, Result};
use prost::Message;
use std::{fs, io::Write, path::PathBuf};

pub fn read_feed(path: &PathBuf) -> Result<Feed> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Feed::decode(&*bytes).with_context(|| format!("failed to decode protobuf: {}", path.display()))
}

pub fn write_feed(path: &PathBuf, feed: Feed) -> Result<Feed> {
    // Ensure parent directory exists (if any)
    if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create directory {}", dir.display()))?;
    }

    let mut buf = Vec::with_capacity(1024);
    feed.encode(&mut buf)?;

    let tmp = path.with_extension("pb.tmp");
    {
        let mut f =
            fs::File::create(&tmp).with_context(|| format!("failed to write {}", tmp.display()))?;
        f.write_all(&buf)?;
        f.flush()?;
    }
    fs::rename(&tmp, path)
        .with_context(|| format!("failed to move temp file into place: {}", path.display()))?;
    Ok(feed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linkleaf_proto::{Feed, Link};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn sample_feed() -> Feed {
        let mut f = Feed::default();
        f.title = "Test".into();
        f.version = 1;
        f.links.push(Link {
            id: "abc123def456".into(),
            title: "Rust".into(),
            url: "https://www.rust-lang.org".into(),
            date: "2025-08-23".into(),
            summary: "The Rust language".into(),
            tags: vec!["rust".into(), "lang".into()],
            via: "".into(),
        });
        f
    }

    #[test]
    fn write_creates_parents_and_roundtrips() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("nested/dir/feed.pb");

        let feed = sample_feed();
        let ret = write_feed(&PathBuf::from(&path), feed.clone())?;
        assert_eq!(ret, feed, "write_feed should return the same feed");
        assert!(path.exists(), "final file should exist");

        // tmp file should have been renamed away
        assert!(
            !path.with_extension("pb.tmp").exists(),
            "tmp file must not remain"
        );

        let read = read_feed(&PathBuf::from(&path))?;
        assert_eq!(read, feed, "read back must equal what we wrote");
        Ok(())
    }

    #[test]
    fn overwrite_updates_content() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("feed.pb");

        let mut f1 = Feed::default();
        f1.title = "One".into();
        f1.version = 1;
        write_feed(&PathBuf::from(&path), f1.clone())?;

        let mut f2 = Feed::default();
        f2.title = "Two".into();
        f2.version = 2;
        write_feed(&PathBuf::from(&path), f2.clone())?;

        let read = read_feed(&PathBuf::from(&path))?;
        assert_eq!(read, f2);
        assert_ne!(read, f1);
        Ok(())
    }

    #[test]
    fn read_missing_file_returns_not_found() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("missing.pb");
        let err = read_feed(&PathBuf::from(&path)).unwrap_err();

        // original io::ErrorKind::NotFound should still be present
        let is_not_found = err
            .downcast_ref::<std::io::Error>()
            .map(|e| e.kind() == std::io::ErrorKind::NotFound)
            .unwrap_or(false);
        assert!(is_not_found, "expected NotFound, got: {err}");

        // and our context message should be included
        assert!(format!("{err}").contains("failed to read"));
    }
}
