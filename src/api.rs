use crate::html::{FeedPage, FeedView, LinkView};
use crate::linkleaf_proto::{Feed, Link};
use anyhow::{Context, Result};
use askama::Template;
use prost::Message;
use std::{fs, io::Write, path::PathBuf};
use time::{Date, OffsetDateTime, UtcOffset, macros::format_description};
use uuid::Uuid;

/// Read a protobuf feed from disk.
///
/// ## Behavior
/// - Reads the entire file at `path` into memory.
/// - Decodes the bytes into a [`Feed`] using `prost`’s `Message::decode`.
///
/// ## Arguments
/// - `path`: Path to the `.pb` file to read.
///
/// ## Returns
/// The decoded [`Feed`] on success.
///
/// ## Errors
/// - I/O errors from [`fs::read`], wrapped with context
///   `"failed to read {path}"`.
/// - Protobuf decode errors from `Feed::decode`, wrapped with context
///   `"failed to decode protobuf: {path}"`.
/// - The error type is [`anyhow::Error`] via your crate-wide `Result`.
///
/// ## Example
/// ```no_run
/// use std::path::PathBuf;
/// use linkleaf::api::read_feed;
/// use anyhow::Result;
///
/// fn main() -> Result<()> {
///     let path = PathBuf::from("mylinks.pb");
///     let feed = read_feed(&path)?;
///     println!("title: {}, links: {}", feed.title, feed.links.len());
///     Ok::<(), anyhow::Error>(())
/// }
/// ```
pub fn read_feed(path: &PathBuf) -> Result<Feed> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Feed::decode(&*bytes).with_context(|| format!("failed to decode protobuf: {}", path.display()))
}

/// Write a protobuf feed to disk **atomically** (best-effort).
///
/// ## Behavior
/// - Ensures the parent directory of `path` exists (creates it if needed).
/// - Encodes `feed` to a temporary file with extension `".pb.tmp"`.
/// - Flushes and then renames the temp file over `path`.
///   - On Unix/POSIX, the rename is atomic when source and destination are on
///     the same filesystem.
///   - On Windows, `rename` may fail if the destination exists; this function
///     forwards that error as-is.
///
/// The input `feed` is consumed and returned unchanged on success to make
/// call sites ergonomic.
///
/// ## Arguments
/// - `path`: Destination path of the `.pb` file.
/// - `feed`: The feed to persist (consumed).
///
/// ## Returns
/// The same [`Feed`] value that was written (handy for chaining).
///
/// ## Errors
/// - Directory creation errors from [`fs::create_dir_all`], with context
///   `"failed to create directory {dir}"`.
/// - File creation/write/flush errors for the temporary file, with context
///   `"failed to write {tmp}"`.
/// - Rename errors when moving the temp file into place, with context
///   `"failed to move temp file into place: {path}"`.
/// - Protobuf encode errors from `feed.encode(&mut buf)`.
/// - The error type is [`anyhow::Error`] via your crate-wide `Result`.
///
/// ## Example
/// ```no_run
/// use std::path::PathBuf;
/// use linkleaf::api::{read_feed, write_feed};
/// use anyhow::Result;
///
/// fn main() -> Result<()> {
///     let path = PathBuf::from("mylinks.pb");
///     let mut feed = read_feed(&path)?;        // or Feed { .. } if creating anew
///     feed.title = "My Links".into();
///     let written = write_feed(&path, feed)?;  // atomic write
///     assert_eq!(written.title, "My Links");
///     Ok(())
/// }
/// ```
///
/// ## Notes
/// - Atomicity requires the temporary file and the destination to be on the
///   **same filesystem**.
/// - If multiple processes may write concurrently, consider adding a file lock
///   around the write section.
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

fn is_not_found(err: &anyhow::Error) -> bool {
    err.downcast_ref::<std::io::Error>()
        .map(|e| e.kind() == std::io::ErrorKind::NotFound)
        .unwrap_or(false)
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

fn update_link_in_place(
    feed: &mut Feed,
    pos: usize,
    title: String,
    url: String,
    date: String,
    summary: Option<String>,
    tags: Option<String>,
    via: Option<String>,
) -> Link {
    // take ownership, mutate, then reinsert at front
    let mut item = feed.links.remove(pos);
    item.title = title;
    item.url = url;
    item.date = date;
    item.summary = summary.unwrap_or_default();
    item.tags = parse_tags(tags);
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
    tags: Option<String>,
    via: Option<String>,
) -> Link {
    let link = Link {
        id,
        title,
        url,
        date,
        summary: summary.unwrap_or_default(),
        tags: parse_tags(tags),
        via: via.unwrap_or_default(),
    };

    let mut new_links = Vec::with_capacity(feed.links.len() + 1);
    new_links.push(link.clone());
    new_links.extend(feed.links.drain(..));
    feed.links = new_links;

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
/// use linkleaf::api::*;
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
pub fn add(
    file: PathBuf,
    title: String,
    url: String,
    summary: Option<String>,
    tags: Option<String>,
    via: Option<String>,
    id: Option<Uuid>,
) -> Result<Link> {
    // compute local timestamp once
    let now = OffsetDateTime::now_utc();
    let offset = UtcOffset::current_local_offset().unwrap();
    let local_now = now.to_offset(offset);
    let fmt = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let date = local_now.format(&fmt).unwrap();

    // read or init feed
    let mut feed = match read_feed(&file) {
        Ok(f) => f,
        Err(err) if is_not_found(&err) => {
            let mut f = Feed::default();
            f.version = 1;
            f
        }
        Err(err) => return Err(err),
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
/// use linkleaf::api::*;
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
                let parsed_date = OffsetDateTime::parse(&l.date, &format).unwrap();
                parsed_date.date() == p
            }
            None => true,
        };

        tag_ok && date_ok
    });

    Ok(feed)
}

/// Render a [`Feed`] into a complete HTML page.
///
/// ## Behavior
/// - Uses `custom_title` if provided; otherwise uses the trimmed feed title,
///   falling back to `"My Links"` when empty.
/// - Maps each link into a lightweight view model (`LinkView`) with:
///   - `has_tags` — whether the link has any tags,
///   - `tags_joined` — comma+space–joined tag string (empty if none).
/// - Wraps the mapped data in a `FeedView` and renders via `FeedPage::render()`.
///
/// This function is purely presentational; it does not mutate or persist the feed.
///
/// ## Arguments
/// - `feed`: The feed to render (consumed by value).
/// - `custom_title`: Optional page title that overrides the feed’s title.
///
/// ## Returns
/// A `String` containing the rendered HTML document.
///
/// ## Errors
/// Returns an [`anyhow::Error`] if rendering fails (the error includes the
/// context `"failed to render HTML"`). No I/O occurs here.
///
/// ## Example
/// ```no_run
/// use linkleaf::api::*;
/// use linkleaf::linkleaf_proto::Feed;
///
/// let feed = Feed { title: "My Links".into(), version: 1, links: vec![] };
/// let page = html(feed, None)?; // Result<String>
/// Ok::<(), anyhow::Error>(())
/// ```
pub fn html(feed: Feed, custom_title: Option<String>) -> Result<String> {
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
        let id = Uuid::parse_str(&a.id).unwrap();
        let _a2 = add(
            file.clone(),
            "A updated".to_string(),
            "https://example.com/a".to_string(),
            None,
            None,
            None,
            Some(id), // ensure it's an update
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
}
