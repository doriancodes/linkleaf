use anyhow::Result;
use tempfile::tempdir;
use uuid::Uuid;

use linkleaf_core::{add, list};

fn main() -> Result<()> {
    let dir = tempdir()?;
    let file = dir.path().join("feed.pb");

    let first = add(
        file.clone(),
        "First",
        "https://one/".into(),
        None,
        Some("alpha".into()),
        None,
        None,
    )?;

    // Update the same logical item by id
    let updated = add(
        file.clone(),
        "First (updated)",
        "https://one-new/".into(),
        Some("note".into()),
        Some("rust,updated".into()),
        Some("hn".into()),
        Some(Uuid::parse_str(&first.id)?),
    )?;

    assert_eq!(updated.id, first.id, "id stays the same on upsert");

    let feed = list(&file, None, None)?;
    println!("links: {}", feed.links.len());
    println!("front item: {} [{}]", feed.links[0].title, feed.links[0].id);

    Ok(())
}
