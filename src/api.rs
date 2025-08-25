use crate::feed::{read_feed, write_feed};
use crate::html::{FeedPage, FeedView, LinkView};
use crate::linkleaf_proto::{Feed, Link};
use anyhow::{Context, Result};
use askama::Template;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use time::OffsetDateTime;

fn is_not_found(err: &anyhow::Error) -> bool {
    err.downcast_ref::<std::io::Error>()
        .map(|e| e.kind() == std::io::ErrorKind::NotFound)
        .unwrap_or(false)
}

fn derive_id(url: &str, date: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    hasher.update(b"|");
    hasher.update(date.as_bytes());
    let digest = hasher.finalize();
    let hexed = hex::encode(digest);
    hexed[..12].to_string()
}

fn parse_tags(raw: Option<String>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect()
    })
    .unwrap_or_default()
}

pub fn add(
    file: PathBuf,
    title: String,
    url: String,
    summary: Option<String>,
    tags: Option<String>,
    via: Option<String>,
    id: Option<String>,
) -> Result<Link> {
    let date = OffsetDateTime::now_utc().date().to_string(); // "YYYY-MM-DD"
    let mut feed = match read_feed(&file) {
        Ok(f) => f,
        Err(err) if is_not_found(&err) => {
            let mut f = Feed::default();
            f.version = 1;
            f
        }
        Err(err) => return Err(err),
    };

    let derived_id = id.unwrap_or_else(|| derive_id(&url, &date));

    if let Some(pos) = feed.links.iter().position(|l| l.id == derived_id) {
        // take ownership
        let mut item = feed.links.remove(pos);
        // mutate
        item.title = title;
        item.url = url;
        item.date = date;
        item.summary = summary.unwrap_or_default();
        item.tags = parse_tags(tags);
        item.via = via.unwrap_or_default();

        // put newest/updated first
        feed.links.insert(0, item.clone());

        write_feed(&file, feed)?;
        eprintln!("Updated existing link (id: {})", item.id);
        return Ok(item);
    }

    let link = Link {
        id: derived_id,
        title,
        url,
        date,
        summary: summary.unwrap_or_default(),
        tags: parse_tags(tags),
        via: via.unwrap_or_default(),
    };

    let mut new_links = Vec::with_capacity(feed.links.len() + 1);
    new_links.push(link.clone());
    new_links.extend(feed.links.into_iter());
    feed.links = new_links;

    let modified_feed = write_feed(&file, feed)?;
    eprintln!(
        "Added link (total {}): {}",
        modified_feed.links.len(),
        file.display()
    );
    Ok(link)
}

pub fn list(file: &PathBuf) -> Result<Feed> {
    let feed = read_feed(file)?;
    Ok(feed)
}

pub fn html<'a>(file: PathBuf, custom_title: Option<String>) -> Result<String> {
    let feed = read_feed(&file)?;

    // map proto → template view; keep it minimal
    let title = custom_title.unwrap_or_else(|| {
        let t = feed.title.trim();
        if t.is_empty() {
            "My Links".into()
        } else {
            t.into()
        }
    });
    let links: Vec<LinkView> = feed
        .links
        .iter()
        .map(|l| {
            let has_tags = !l.tags.is_empty();
            let tags_joined = if has_tags {
                l.tags.join(", ")
            } else {
                String::new()
            };
            LinkView {
                title: l.title.clone(),
                url: l.url.clone(),
                date: l.date.clone(),
                summary: l.summary.clone(),
                via: l.via.clone(),
                has_tags,
                tags_joined,
            }
        })
        .collect();

    let view = FeedView {
        title,
        count: links.len(),
        links,
    };
    let page = FeedPage { feed: &view };
    let html = page.render().context("failed to render HTML")?;

    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*; // bring `add`, `read_feed`, `Link`, etc. into scope
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_feed_path() -> PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("linkleaf_test_{}_{}.pb", std::process::id(), nanos));
        p
    }

    #[test]
    fn updated_link_moves_to_front() {
        let file = unique_feed_path();
        // start clean
        let _ = std::fs::remove_file(&file);

        // 1) add two links
        let a = add(
            file.clone(),
            "A".to_string(),
            "https://example.com/a".to_string(),
            None,
            None,
            None,
            None,
        )
        .expect("add A");

        let _b = add(
            file.clone(),
            "B".to_string(),
            "https://example.com/b".to_string(),
            None,
            None,
            None,
            None,
        )
        .expect("add B");

        // 2) update A (by id) with a new title
        let _a2 = add(
            file.clone(),
            "A updated".to_string(),
            "https://example.com/a".to_string(),
            None,
            None,
            None,
            Some(a.id.clone()), // ensure it's an update
        )
        .expect("update A");

        // 3) verify order: updated A is first
        let feed = read_feed(&file).expect("read after update");
        assert!(!feed.links.is_empty(), "feed should not be empty");
        assert_eq!(feed.links[0].id, a.id, "updated link should be first");
        assert_eq!(
            feed.links[0].title, "A updated",
            "title should reflect the update"
        );

        // cleanup (best-effort)
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn parse_tags_various_whitespace() {
        assert_eq!(super::parse_tags(None), Vec::<String>::new());
        assert_eq!(
            super::parse_tags(Some("a, b,  ,c".into())),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn derive_id_is_stable() {
        let a = super::derive_id("https://x", "2025-08-23");
        let b = super::derive_id("https://x", "2025-08-23");
        assert_eq!(a, b);
        assert_eq!(a.len(), 12);
    }
}
