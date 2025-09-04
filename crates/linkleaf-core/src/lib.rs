pub mod fs;
pub mod validation;
pub mod linkleaf_proto {
    include!(concat!(env!("OUT_DIR"), "/linkleaf.v1.rs"));
}

use crate::fs::{read_feed, write_feed};
use crate::linkleaf_proto::{Feed, Link};
use crate::validation::parse_tags;
use anyhow::Result;
use std::path::Path;
use std::path::PathBuf;
use time::{Date, OffsetDateTime, PrimitiveDateTime, macros::format_description};
use uuid::Uuid;

const TS_FMT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

fn is_not_found(err: &anyhow::Error) -> bool {
    err.downcast_ref::<std::io::Error>()
        .map(|e| e.kind() == std::io::ErrorKind::NotFound)
        .unwrap_or(false)
}

fn update_link_in_place(
    feed: &mut Feed,
    pos: usize,
    title: String,
    url: String,
    date: String,
    summary: Option<String>,
    tags: Vec<String>,
    via: Option<String>,
) -> Link {
    // take ownership, mutate, then reinsert at front
    let mut item = feed.links.remove(pos);
    item.title = title;
    item.url = url;
    item.date = date;
    item.summary = summary.unwrap_or_default();
    item.tags = tags;
    item.via = via.unwrap_or_default();

    feed.links.insert(0, item.clone());
    item
}

fn insert_new_link_front(
    feed: &mut Feed,
    id: String,
    title: String,
    url: String,
    date: String,
    summary: Option<String>,
    tags: Vec<String>,
    via: Option<String>,
) -> Link {
    let link = Link {
        summary: summary.unwrap_or_default(),
        tags, // field init shorthand
        via: via.unwrap_or_default(),
        id,
        title,
        url,
        date,
    };
    feed.links.insert(0, link.clone());
    link
}

/// Add or update a link in a protobuf feed file, then persist the feed.
///
/// ## Behavior
/// - Tries to read the feed at `file`. If the file does not exist, a new feed is
///   initialized (`version = 1`).
/// - If `id` or `url` matches an existing link, that link is **updated** (title, url,
///   summary, tags, via) and its `date` is set to **today (local datetime, `YYYY-MM-DD HH:MM:SS`)**.
///   The updated link is moved to the **front** of the list (newest-first).
/// - Otherwise a new link is **inserted at the front**. Its `id` is
///   `Uuid::new_v4()` unless `id` is provided. If the provided `id` is not a valid uuid,
///   the function returns an error.
///
/// Persists the whole feed by calling `write_feed`, which writes atomically
/// via a temporary file + rename.
///
/// ## Arguments
/// - `file`: Path to the `.pb` feed file to update/create.
/// - `title`: Human-readable title for the link.
/// - `url`: Target URL for the link.
/// - `summary`: Optional blurb/notes (empty string if `None`).
/// - `tags`: Optional tag list as a single string; parsed by `parse_tags`
///   (e.g. `"rust, async, tokio"` → `["rust","async","tokio"]`).
/// - `via`: Optional source/attribution (empty if `None`).
/// - `id`: Optional stable identifier. If present, performs an **upsert** of that
///   item. If absent, it generates a UUID v4.
///
/// ## Returns
/// The newly created or updated [`Link`].
///
/// ## Ordering
/// Links are kept **newest-first**; both inserts and updates end up at index `0`.
///
/// ## Errors
/// - Any error from `read_feed` (except “not found”) is returned.
/// - Any error from `write_feed` is returned.
/// - This function performs no inter-process locking; concurrent writers may race.
///
/// ## Example
/// ```no_run
/// use std::path::PathBuf;
/// use linkleaf_core::*;
/// use uuid::Uuid;
///
/// let file = PathBuf::from("mylinks.pb");
///
/// // Create a new link
/// let a = add(
///     file.clone(),
///     "Tokio - Asynchronous Rust".into(),
///     "https://tokio.rs/".into(),
///     None,
///     Some("rust, async, tokio".into()),
///     None,
///     None, // no id -> create
/// )?;
///
/// // Update the same link by id (upsert)
/// let _id = Uuid::parse_str(&a.id)?;
/// let a2 = add(
///     file.clone(),
///     "Tokio • Async Rust".into(),
///     "https://tokio.rs/".into(),
///     Some("A runtime for reliable async apps".into()),
///     None,
///     None,
///     Some(_id), // provide id -> update
/// )?;
///
/// assert_eq!(a2.id, a.id);
/// Ok::<(), anyhow::Error>(())
/// // After update, the item is at the front (index 0).
/// ```
///
/// ## Notes
/// - Using a provided `id` gives you a stable identity and it is tied to the url.
/// - `date` is always set to today (local time) on both create and update.
pub fn add<P: AsRef<Path>>(
    file: P,
    title: String,
    url: String,
    summary: Option<String>,
    tags: Option<String>,
    via: Option<String>,
    id: Option<Uuid>,
) -> Result<Link> {
    let file = file.as_ref();
    // compute local timestamp once
    let local_now = OffsetDateTime::now_local()
        .map_err(|e| anyhow::anyhow!("failed to get local time offset: {e}"))?;
    let date = local_now
        .format(TS_FMT)
        .map_err(|e| anyhow::anyhow!("failed to format timestamp: {e}"))?;

    // read or init feed
    let mut feed = match read_feed(file) {
        Ok(f) => f,
        Err(err) if is_not_found(&err) => {
            let mut f = Feed::default();
            f.version = 1;
            f
        }
        Err(err) => return Err(err),
    };

    let tags = match tags.as_deref() {
        Some(s) => parse_tags(s).map_err(|e| anyhow::anyhow!("invalid tags: {e}"))?,
        None => Vec::new(),
    };

    // behavior:
    // - If `id` provided: update by id; else insert (even if URL duplicates).
    // - If no `id`: update by URL; else insert with fresh UUID.
    let updated_or_new = match id {
        Some(uid) => {
            let uid_str = uid.to_string();
            if let Some(pos) = feed.links.iter().position(|l| l.id == uid_str) {
                let item =
                    update_link_in_place(&mut feed, pos, title, url, date, summary, tags, via);
                eprintln!("Updated existing link (id: {})", item.id);
                item
            } else {
                let item =
                    insert_new_link_front(&mut feed, uid_str, title, url, date, summary, tags, via);
                eprintln!("Inserted new link with explicit id: {}", item.id);
                item
            }
        }
        None => {
            if let Some(pos) = feed.links.iter().position(|l| l.url == url) {
                let item =
                    update_link_in_place(&mut feed, pos, title, url, date, summary, tags, via);
                eprintln!("Updated existing link (url: {})", item.url);
                item
            } else {
                let uid = Uuid::new_v4().to_string();
                let item =
                    insert_new_link_front(&mut feed, uid, title, url, date, summary, tags, via);
                eprintln!("Inserted new link with generated id: {}", item.id);
                item
            }
        }
    };

    let modified_feed = write_feed(&file, feed)?;
    eprintln!(
        "Feed now has {} link(s): {}",
        modified_feed.links.len(),
        file.display()
    );

    Ok(updated_or_new)
}

/// Read and return the feed stored in a protobuf file.
///
/// ## Behavior
/// Calls [`read_feed`] on the provided path and returns the parsed [`Feed`]. If tags and/or
/// date filters are provided it filters the resulting [`Feed`].
///
/// ## Arguments
/// - `file`: Path to the `.pb` feed file.
///
/// ## Returns
/// The parsed [`Feed`] on success.
///
/// ## Errors
/// Any error bubbled up from [`read_feed`], e.g. I/O errors (file missing,
/// permissions), or decode errors if the file is not a valid feed.
///
/// ## Example
/// ```no_run
/// use std::path::PathBuf;
/// use linkleaf_core::*;
///
/// let path = PathBuf::from("mylinks.pb");
/// let feed = list(&path, None, None)?;
/// println!("Title: {}, links: {}", feed.title, feed.links.len());
/// Ok::<(), anyhow::Error>(())
/// ```
pub fn list(file: &PathBuf, tags: Option<Vec<String>>, date: Option<Date>) -> Result<Feed> {
    let mut feed = read_feed(file)?;

    let tag_norms: Option<Vec<String>> = tags.map(|ts| {
        ts.iter()
            .map(|t| t.trim().to_ascii_lowercase())
            .filter(|t| !t.is_empty())
            .collect()
    });

    feed.links.retain(|l| {
        let tag_ok = match &tag_norms {
            Some(needles) => l
                .tags
                .iter()
                .any(|t| needles.iter().any(|n| t.eq_ignore_ascii_case(n))),
            None => true,
        };

        let date_ok = match date {
            Some(p) => {
                let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
                let parsed_date =
                    PrimitiveDateTime::parse(&l.date, &format).expect("Failed to parse date.");
                parsed_date.date() == p
            }
            None => true,
        };

        tag_ok && date_ok
    });

    Ok(feed)
}

#[cfg(test)]
mod tests {
    use super::{add, list};
    use crate::fs::{read_feed, write_feed};
    use crate::linkleaf_proto::{Feed, Link};
    use anyhow::Result;
    use tempfile::tempdir;
    use time::macros::date;
    use uuid::Uuid;

    // ---- helpers -------------------------------------------------------------

    fn mk_link(
        id: &str,
        title: &str,
        url: &str,
        date_s: &str,
        tags: &[&str],
        summary: &str,
        via: &str,
    ) -> Link {
        Link {
            id: id.to_string(),
            title: title.to_string(),
            url: url.to_string(),
            date: date_s.to_string(),
            summary: summary.to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            via: via.to_string(),
        }
    }

    fn mk_feed(links: Vec<Link>) -> Feed {
        let mut f = Feed::default();
        f.version = 1;
        f.links = links;
        f
    }

    // ---- tests ---------------------------------------------------------------

    #[test]
    fn add_creates_file_and_initializes_feed() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("feed.pb");

        // via=None & tags string -> defaults + parse_tags used internally
        let created = add(
            file.clone(),
            "Tokio".into(),
            "https://tokio.rs/".into(),
            None,                               // summary -> ""
            Some("rust, async , tokio".into()), // gets trimmed/split
            None,                               // via -> ""
            None,                               // id -> generated
        )?;

        // File exists and can be read; version initialized to 1
        let feed = read_feed(&file)?;
        assert_eq!(feed.version, 1);
        assert_eq!(feed.links.len(), 1);
        let l = &feed.links[0];
        assert_eq!(l.id, created.id);
        assert_eq!(l.title, "Tokio");
        assert_eq!(l.url, "https://tokio.rs/");
        assert_eq!(l.summary, "");
        assert_eq!(l.via, "");
        assert_eq!(l.tags, vec!["rust", "async", "tokio"]);

        // ID is a valid UUID
        let _ = Uuid::parse_str(&created.id).expect("id should be a valid UUID");
        Ok(())
    }

    #[test]
    fn add_with_explicit_id_inserts_with_given_id() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("feed.pb");
        let wanted = Uuid::new_v4();

        let created = add(
            file.clone(),
            "A".into(),
            "https://a.example/".into(),
            Some("hi".into()),
            Some("x,y".into()),
            Some("via".into()),
            Some(wanted),
        )?;

        assert_eq!(created.id, wanted.to_string());

        // list(None, None) returns everything; first item is the one we just added
        let feed = list(&file, None, None)?;
        assert_eq!(feed.links.len(), 1);
        assert_eq!(feed.links[0].id, wanted.to_string());
        Ok(())
    }

    #[test]
    fn add_update_by_id_moves_to_front_and_updates_fields() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("feed.pb");

        // Seed with two links
        let a = add(
            file.clone(),
            "First".into(),
            "https://one/".into(),
            None,
            Some("alpha".into()),
            None,
            None,
        )?;
        let _b = add(
            file.clone(),
            "Second".into(),
            "https://two/".into(),
            None,
            Some("beta".into()),
            None,
            None,
        )?;

        // Update by id of 'a': title/url/tags/via/summary overwritten, item moves to front
        let updated = add(
            file.clone(),
            "First (updated)".into(),
            "https://one-new/".into(),
            Some("note".into()),
            Some("rust,updated".into()),
            Some("HN".into()),
            Some(Uuid::parse_str(&a.id)?),
        )?;
        assert_eq!(updated.id, a.id);
        assert_eq!(updated.title, "First (updated)");
        assert_eq!(updated.url, "https://one-new/");
        assert_eq!(updated.summary, "note");
        assert_eq!(updated.via, "HN");
        assert_eq!(updated.tags, vec!["rust", "updated"]);

        let feed = list(&file, None, None)?;
        assert_eq!(feed.links.len(), 2);
        assert_eq!(feed.links[0].id, a.id, "updated item should be at index 0");
        assert_eq!(feed.links[0].title, "First (updated)");
        Ok(())
    }

    #[test]
    fn add_update_by_url_when_id_absent() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("feed.pb");

        let first = add(
            file.clone(),
            "Original".into(),
            "https://same.url/".into(),
            None,
            None,
            None,
            None,
        )?;

        // Same URL, id=None => update-in-place (but moved to front) and id stays the same
        let updated = add(
            file.clone(),
            "Original (updated)".into(),
            "https://same.url/".into(),
            Some("s".into()),
            Some("t1,t2".into()),
            None,
            None,
        )?;
        assert_eq!(updated.id, first.id);

        let feed = list(&file, None, None)?;
        assert_eq!(feed.links.len(), 1);
        assert_eq!(feed.links[0].title, "Original (updated)");
        assert_eq!(feed.links[0].tags, vec!["t1", "t2"]);
        Ok(())
    }

    #[test]
    fn add_inserts_new_when_url_diff_and_id_absent() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("feed.pb");

        let _a = add(
            file.clone(),
            "A".into(),
            "https://a/".into(),
            None,
            None,
            None,
            None,
        )?;
        let b = add(
            file.clone(),
            "B".into(),
            "https://b/".into(),
            None,
            None,
            None,
            None,
        )?;

        let feed = list(&file, None, None)?;
        assert_eq!(feed.links.len(), 2);
        assert_eq!(feed.links[0].id, b.id, "new item should be at front");
        Ok(())
    }

    #[test]
    fn add_returns_error_on_corrupt_feed() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("feed.pb");

        // Write junk so read_feed(file) inside add() fails with decode error.
        std::fs::write(&file, b"not a protobuf")?;

        let err = add(
            file.clone(),
            "X".into(),
            "https://x/".into(),
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        // Just assert it is an error; message content is from read_feed context.
        assert!(!err.to_string().is_empty());
        Ok(())
    }

    #[test]
    fn list_without_filters_returns_all() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("feed.pb");

        // Build a feed directly so we control dates/tags precisely
        let l1 = mk_link(
            "1",
            "One",
            "https://1/",
            "2025-01-02 12:00:00",
            &["rust", "async"],
            "",
            "",
        );
        let l2 = mk_link(
            "2",
            "Two",
            "https://2/",
            "2025-01-03 09:30:15",
            &["tokio"],
            "",
            "",
        );
        write_feed(&file, mk_feed(vec![l2.clone(), l1.clone()]))?;

        let feed = list(&file, None, None)?;
        assert_eq!(feed.links.len(), 2);
        // Order is preserved from the stored feed for list()
        assert_eq!(feed.links[0].id, l2.id);
        assert_eq!(feed.links[1].id, l1.id);
        Ok(())
    }

    #[test]
    fn list_filters_by_tag_case_insensitive_any_match() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("feed.pb");

        let l1 = mk_link(
            "1",
            "One",
            "https://1/",
            "2025-01-02 12:00:00",
            &["rust", "async"],
            "",
            "",
        );
        let l2 = mk_link(
            "2",
            "Two",
            "https://2/",
            "2025-01-03 09:30:15",
            &["Tokio"], // mixed case
            "",
            "",
        );
        write_feed(&file, mk_feed(vec![l1.clone(), l2.clone()]))?;

        // ANY-of semantics, case-insensitive
        let feed_tokio = list(&file, Some(vec!["tokio".into()]), None)?;
        assert_eq!(feed_tokio.links.len(), 1);
        assert_eq!(feed_tokio.links[0].id, l2.id);

        let feed_async = list(&file, Some(vec!["ASYNC".into()]), None)?;
        assert_eq!(feed_async.links.len(), 1);
        assert_eq!(feed_async.links[0].id, l1.id);

        // Multiple needles -> still "any"
        let feed_multi = list(&file, Some(vec!["zzz".into(), "rust".into()]), None)?;
        assert_eq!(feed_multi.links.len(), 1);
        assert_eq!(feed_multi.links[0].id, l1.id);

        Ok(())
    }

    #[test]
    fn list_filters_by_exact_date_component() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("feed.pb");

        let l1 = mk_link(
            "1",
            "Jan02",
            "https://1/",
            "2025-01-02 00:00:00",
            &[],
            "",
            "",
        );
        let l2 = mk_link(
            "2",
            "Jan03",
            "https://2/",
            "2025-01-03 23:59:59",
            &[],
            "",
            "",
        );
        write_feed(&file, mk_feed(vec![l1.clone(), l2.clone()]))?;

        let filtered = list(&file, None, Some(date!(2025 - 01 - 03)))?;
        assert_eq!(filtered.links.len(), 1);
        assert_eq!(filtered.links[0].id, l2.id);

        let filtered2 = list(&file, None, Some(date!(2025 - 01 - 02)))?;
        assert_eq!(filtered2.links.len(), 1);
        assert_eq!(filtered2.links[0].id, l1.id);

        Ok(())
    }
}
