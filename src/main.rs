use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use askama::Template;
use clap::{Args, Parser, Subcommand};
use linkleaf_rs::feed::{read_feed, write_feed};
use linkleaf_rs::linkleaf_proto::{Feed, Link};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Write};
use time::OffsetDateTime;

#[derive(Parser)]
#[command(name = "linkleaf", about = "protobuf-only feed manager (linkleaf.v1)")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new feed file
    Init(InitArgs),

    /// Add a link to the feed
    Add(AddArgs),

    /// List links (compact)
    List(FileArg),

    /// Print links (detailed)
    Print(FileArg),

    /// Render HTML from the feed
    Html(HtmlArgs),
}

#[derive(Args)]
struct FileArg {
    /// Path to the feed .pb file
    #[arg(value_name = "FILE", default_value = "feed/mylinks.pb")]
    file: PathBuf,
}

#[derive(Args)]
struct InitArgs {
    /// Path to create the feed .pb file
    #[arg(value_name = "FILE", default_value = "feed/mylinks.pb")]
    file: PathBuf,

    /// Feed title
    #[arg(short, long, default_value = "My Links")]
    title: String,

    /// Feed version
    #[arg(short, long, default_value = "1")]
    version: u32,
}

#[derive(Args)]
struct AddArgs {
    /// Path to the feed .pb file
    #[arg(value_name = "FILE", default_value = "feed/mylinks.pb")]
    file: PathBuf,

    /// Link title
    #[arg(short, long)]
    title: String,

    /// Link URL
    #[arg(short, long)]
    url: String,

    /// Optional summary
    #[arg(short, long)]
    summary: Option<String>,

    /// Optional comma-separated tags
    #[arg(short = 'g', long)]
    tags: Option<String>,

    /// Optional "via" URL
    #[arg(long)]
    via: Option<String>,

    /// Override auto id (defaults to sha256(url|date)[:12])
    #[arg(long)]
    id: Option<String>,
}

#[derive(Args)]
struct HtmlArgs {
    /// Input feed .pb file
    #[arg(value_name = "FILE", default_value = "feed/mylinks.pb")]
    file: PathBuf,

    /// Output HTML file (e.g., docs/index.html)
    #[arg(short, long, default_value = "assets/index.html")]
    out: PathBuf,

    /// Page title (defaults to feed.title)
    #[arg(short, long)]
    title: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init(args) => cmd_init(args.file, args.title, args.version),
        Commands::Add(args) => cmd_add(
            args.file,
            args.title,
            args.url,
            args.summary,
            args.tags,
            args.via,
            args.id,
        ),
        Commands::List(args) => cmd_list(args.file),
        Commands::Print(args) => cmd_print(args.file),
        Commands::Html(args) => cmd_html(args.file, args.out, args.title),
    }
}

fn cmd_init(file: PathBuf, title: String, version: u32) -> Result<()> {
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

fn cmd_add(
    file: PathBuf,
    title: String,
    url: String,
    summary: Option<String>,
    tags: Option<String>,
    via: Option<String>,
    id: Option<String>,
) -> Result<()> {
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
    new_links.push(link);
    new_links.extend(feed.links.into_iter());
    feed.links = new_links;

    let modified_feed = write_feed(&file, feed)?;
    eprintln!(
        "Added link (total {}): {}",
        modified_feed.links.len(),
        file.display()
    );
    Ok(())
}

fn cmd_list(file: PathBuf) -> Result<()> {
    let feed = read_feed(&file)?;
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
    Ok(())
}

fn cmd_print(file: PathBuf) -> Result<()> {
    let feed = read_feed(&file)?;
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
    Ok(())
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

fn derive_id(url: &str, date: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    hasher.update(b"|");
    hasher.update(date.as_bytes());
    let digest = hasher.finalize();
    let hexed = hex::encode(digest);
    hexed[..12].to_string()
}

fn is_not_found(err: &anyhow::Error) -> bool {
    err.downcast_ref::<std::io::Error>()
        .map(|e| e.kind() == std::io::ErrorKind::NotFound)
        .unwrap_or(false)
}

#[derive(serde::Serialize)]
struct LinkView {
    title: String,
    url: String,
    date: String,
    summary: String,
    via: String,
    has_tags: bool,
    tags_joined: String,
}

#[derive(Serialize)]
struct FeedView {
    title: String,
    count: usize,
    links: Vec<LinkView>,
}

#[derive(Template)]
#[template(path = "feed.html", escape = "html")]
struct FeedPage<'a> {
    feed: &'a FeedView,
}

fn cmd_html(file: PathBuf, out: PathBuf, custom_title: Option<String>) -> Result<()> {
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

    // write atomically (same pattern as write_feed)
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).ok();
    }
    let tmp = out.with_extension("html.tmp");
    {
        let mut f =
            fs::File::create(&tmp).with_context(|| format!("failed to write {}", tmp.display()))?;
        f.write_all(html.as_bytes())?;
        f.flush()?;
    }
    fs::rename(&tmp, &out)
        .with_context(|| format!("failed to move temp file into place: {}", out.display()))?;

    // 🔹 copy style.css next to the HTML output
    let css_src = PathBuf::from("templates/style.css");
    if css_src.exists() {
        let css_out = out
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("style.css");
        fs::copy(&css_src, &css_out).with_context(|| {
            format!(
                "failed to copy {} to {}",
                css_src.display(),
                css_out.display()
            )
        })?;
    }

    eprintln!("Wrote HTML: {}", out.display());
    Ok(())
}
