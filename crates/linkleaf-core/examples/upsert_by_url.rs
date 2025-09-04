use anyhow::Result;
use tempfile::tempdir;

use linkleaf_core::{add, list};

fn main() -> Result<()> {
    let dir = tempdir()?;
    let file = dir.path().join("feed.pb");

    let a = add(
        file.clone(),
        "Original".into(),
        "https://same.url/".into(),
        None,
        Some("t1".into()),
        None,
        None,
    )?;

    // Same URL + id=None -> update the existing entry (moved to front)
    let a2 = add(
        file.clone(),
        "Original (updated)".into(),
        "https://same.url/".into(),
        Some("updated".into()),
        Some("t2".into()),
        None,
        None,
    )?;

    assert_eq!(a.id, a2.id);

    let feed = list(&file, None, None)?;
    println!(
        "front item: {} [{}] tags: {:?}",
        feed.links[0].title, feed.links[0].id, feed.links[0].tags
    );

    Ok(())
}
