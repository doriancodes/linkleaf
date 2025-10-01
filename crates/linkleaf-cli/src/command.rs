use anyhow::{Result, bail};
use linkleaf_core::fs::write_feed;
use linkleaf_core::linkleaf_proto::Feed;
use linkleaf_core::{add, list};
use std::path::PathBuf;
use time::Date;
use uuid::Uuid;

pub fn cmd_init(file: PathBuf, title: String, version: u32) -> Result<()> {
    if file.exists() {
        bail!("file already exists: {}", file.display());
    }

    let mut feed = Feed::default();
    feed.title = title;
    feed.version = version;

    let modified_feed = write_feed(&file, feed)?;
    eprintln!(
        "Initialized feed: '{}' (v{}) → {}",
        modified_feed.title,
        modified_feed.version,
        file.display()
    );
    Ok(())
}

pub fn cmd_add(
    file: PathBuf,
    title: String,
    url: String,
    summary: Option<String>,
    tags: Option<String>,
    via: Option<String>,
    id: Option<Uuid>,
) -> Result<()> {
    add(file, title, url, summary, tags, via, id)?;
    Ok(())
}

pub fn cmd_list(
    file: PathBuf,
    long: bool,
    tags: Option<Vec<String>>,
    date: Option<Date>,
) -> Result<()> {
    let feed = list(&file, tags, date)?;

    if long {
        long_print(feed);
    } else {
        println!(
            "Feed: '{}' (v{}) — {} links\n",
            feed.title,
            feed.version,
            feed.links.len()
        );

        for (idx, l) in feed.links.iter().enumerate() {
            let tags = if l.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", l.tags.join(","))
            };
            println!(
                "{:>3}. {}  {}{}\n     {}",
                idx + 1,
                l.date,
                l.title,
                tags,
                l.url
            );
        }
    }
    Ok(())
}

fn long_print(feed: Feed) {
    println!("Feed: '{}' (v{})\n", feed.title, feed.version);
    for l in &feed.links {
        println!("- [{}] {}", l.date, l.title);
        println!("  id: {}", l.id);
        println!("  url: {}", l.url);
        if !l.via.is_empty() {
            println!("  via: {}", l.via);
        }
        if !l.summary.is_empty() {
            println!("  summary: {}", l.summary);
        }
        if !l.tags.is_empty() {
            println!("  tags: {}", l.tags.join(", "));
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkleaf_core::fs::read_feed;
    use linkleaf_core::linkleaf_proto::{Feed, Link};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn sample_feed_one() -> Feed {
        let mut f = Feed::default();
        f.title = "Sample".into();
        f.version = 1;
        f.links.push(Link {
            id: "one".into(),
            title: "First".into(),
            url: "https://example.com/1".into(),
            date: "2025-01-01".into(),
            summary: "hello".into(),
            tags: vec!["x".into(), "y".into()],
            via: "".into(),
        });
        f
    }

    #[test]
    fn init_creates_file_and_defaults() -> anyhow::Result<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("nested/dir/mylinks.pb");
        cmd_init(path.clone(), "My Links".into(), 2)?;
        assert!(path.exists(), "feed file should exist");
        let feed = read_feed(&PathBuf::from(&path))?;
        assert_eq!(feed.title, "My Links");
        assert_eq!(feed.version, 2);
        assert!(feed.links.is_empty());
        Ok(())
    }

    #[test]
    fn add_inserts_then_updates_same_id() -> anyhow::Result<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("feed.pb");

        let _id = Uuid::new_v4();

        // Insert (file doesn't exist yet; implementation should create a v1 feed)
        cmd_add(
            path.clone(),
            "Rust Book".into(),
            "https://doc.rust-lang.org/book/".into(),
            Some("Great read".into()),
            Some("rust,book".into()),
            Some("https://rust-lang.org".into()),
            Some(_id.clone()), // ensure deterministic update target
        )?;
        let mut feed = read_feed(&PathBuf::from(&path))?;
        assert_eq!(feed.links.len(), 1);
        assert_eq!(feed.links[0].id, _id.clone().to_string());
        assert_eq!(feed.links[0].title, "Rust Book");

        // Update same id: title & summary change; still one entry
        cmd_add(
            path.clone(),
            "The Rust Book".into(),
            "https://doc.rust-lang.org/book/".into(),
            Some("Updated summary".into()),
            Some("rust,book".into()),
            None,
            Some(_id.into()),
        )?;
        feed = read_feed(&PathBuf::from(&path))?;
        assert_eq!(feed.links.len(), 1, "should update, not duplicate");
        assert_eq!(feed.links[0].title, "The Rust Book");
        assert_eq!(feed.links[0].summary, "Updated summary");
        Ok(())
    }

    #[test]
    fn list_compact_and_long_run() -> anyhow::Result<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("feed.pb");
        write_feed(&PathBuf::from(&path), sample_feed_one())?;

        // We don’t assert output formatting here; just ensure no panic/err.
        cmd_list(path.clone(), false, None, None)?;
        cmd_list(path.clone(), true, None, None)?;
        Ok(())
    }
}
