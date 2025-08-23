use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use linkleaf_rs::feed::{read_feed, write_feed};
use linkleaf_rs::linkleaf_proto::{Feed, Link};
use sha2::{Digest, Sha256};
use time::Date;
use time::macros::format_description;

#[derive(Parser)]
#[command(name = "linkleaf", about = "protobuf-only feed manager (linkleaf.v1)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        file: PathBuf,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        version: Option<u32>,
    },
    Add {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(short, long)]
        title: String,
        #[arg(short, long)]
        url: String,
        #[arg(short, long)]
        date: String,
        #[arg(short, long)]
        summary: Option<String>,
        #[arg(short, long)]
        tags: Option<String>,
        #[arg(long)]
        via: Option<String>,
        #[arg(long)]
        id: Option<String>,
    },
    List {
        file: PathBuf,
    },
    Print {
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init {
            file,
            title,
            version,
        } => cmd_init(file, title, version),
        Commands::Add {
            file,
            title,
            url,
            date,
            summary,
            tags,
            via,
            id,
        } => cmd_add(file, title, url, date, summary, tags, via, id),
        Commands::List { file } => cmd_list(file),
        Commands::Print { file } => cmd_print(file),
    }
}

fn cmd_init(file: PathBuf, title: Option<String>, version: Option<u32>) -> Result<()> {
    if file.exists() {
        bail!("file already exists: {}", file.display());
    }

    let mut feed = Feed::default();
    feed.title = title.unwrap_or_else(|| "My Links".to_string());
    feed.version = version.unwrap_or(1);

    write_feed(&file, &feed)?;
    eprintln!(
        "Initialized feed: '{}' (v{}) → {}",
        feed.title,
        feed.version,
        file.display()
    );
    Ok(())
}

fn cmd_add(
    file: PathBuf,
    title: String,
    url: String,
    date: String,
    summary: Option<String>,
    tags: Option<String>,
    via: Option<String>,
    id: Option<String>,
) -> Result<()> {
    validate_date(&date)?;

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

    write_feed(&file, &feed)?;
    eprintln!(
        "Added link (total {}): {}",
        feed.links.len(),
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

fn validate_date(s: &str) -> Result<()> {
    let fmt = format_description!("[year]-[month]-[day]");
    let _ = Date::parse(s, &fmt)
        .with_context(|| format!("invalid date (expected YYYY-MM-DD): {}", s))?;
    Ok(())
}

fn is_not_found(err: &anyhow::Error) -> bool {
    err.downcast_ref::<std::io::Error>()
        .map(|e| e.kind() == std::io::ErrorKind::NotFound)
        .unwrap_or(false)
}
