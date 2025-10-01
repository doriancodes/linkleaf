use anyhow::Result;
use tempfile::tempdir;

use linkleaf_core::{add, feed_to_rss_xml, list};

fn main() -> Result<()> {
    let dir = tempdir()?;
    let file = dir.path().join("feed.pb");

    let _a = add(
        file.clone(),
        "Tokio - Asynchronous Rust",
        "https://tokio.rs/".into(),
        Some("A runtime for reliable async apps".into()),
        Some("rust, async, tokio".into()),
        Some("website".into()),
        None, // generate id
    )?;

    let feed = list(&file, None, None)?;

    let rss_feed = feed_to_rss_xml(&feed, "", "")?;

    println!("{}", rss_feed);

    Ok(())
}
