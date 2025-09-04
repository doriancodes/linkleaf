use anyhow::Result;
use tempfile::tempdir;

use linkleaf_core::{add, list};
use time::{OffsetDateTime, UtcOffset};

fn main() -> Result<()> {
    let dir = tempdir()?;
    let file = dir.path().join("feed.pb");

    let _a = add(
        file.clone(),
        "Tokio - Asynchronous Rust".into(),
        "https://tokio.rs/".into(),
        Some("A runtime for reliable async apps".into()),
        Some("rust, async, tokio".into()),
        Some("website".into()),
        None, // generate id
    )?;

    // list everything
    let feed = list(&file, None, None)?;
    println!("feed version: {}", feed.version);
    println!("links: {}", feed.links.len());
    for (i, l) in feed.links.iter().enumerate() {
        println!("{i}: {} [{}]  {}", l.title, l.id, l.url);
    }

    // show today's date we wrote (for reference)
    let today = OffsetDateTime::now_utc()
        .to_offset(UtcOffset::current_local_offset()?)
        .date();
    println!("today (local): {today}");

    Ok(())
}
