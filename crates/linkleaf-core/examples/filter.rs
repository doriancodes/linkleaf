use anyhow::Result;
use tempfile::tempdir;

use linkleaf_core::{add, list};
use time::{OffsetDateTime, UtcOffset};

fn main() -> Result<()> {
    let dir = tempdir()?;
    let file = dir.path().join("feed.pb");

    // Seed some links (dates are set to "now local" internally)
    let _ = add(
        file.clone(),
        "A",
        "https://a/".into(),
        None,
        Some("rust, async".into()),
        None,
        None,
    )?;
    let _ = add(
        file.clone(),
        "B",
        "https://b/".into(),
        None,
        Some("tokio".into()),
        None,
        None,
    )?;
    let _ = add(
        file.clone(),
        "C",
        "https://c/".into(),
        None,
        Some("db, rust".into()),
        None,
        None,
    )?;

    // Filter by tag (case-insensitive, any-of)
    let rust_only = list(&file, Some(vec!["RUST".into()]), None)?;
    println!("rust_only: {}", rust_only.links.len());
    for l in &rust_only.links {
        println!("- {}", l.title);
    }

    // Filter by today's date
    let today = OffsetDateTime::now_utc()
        .to_offset(UtcOffset::current_local_offset()?)
        .date();
    let today_only = list(&file, None, Some(today))?;
    println!("today_only: {}", today_only.links.len());

    Ok(())
}
