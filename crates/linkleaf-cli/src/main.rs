mod command;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use command::{cmd_add, cmd_html, cmd_init, cmd_list, cmd_publish};
use linkleaf_core::validation::{parse_date, parse_tags};
use std::path::PathBuf;
use time::Date;
use uuid::Uuid;

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

    /// List links (compact by default; use -l/--long for details)
    List(ListArgs),

    /// Render HTML from the feed
    Html(HtmlArgs),

    /// Commit & push the feed file to a git remote
    Publish(PublishArgs),
}

#[derive(Args)]
struct PublishArgs {
    /// Path to the feed .pb file
    #[arg(value_name = "FILE", default_value = "feed/mylinks.pb")]
    file: PathBuf,

    /// Git remote name
    #[arg(short, long, default_value = "origin")]
    remote: String,

    /// Branch to push to; if omitted, uses the current upstream
    #[arg(short = 'b', long)]
    branch: Option<String>,

    /// Commit message
    #[arg(short = 'm', long, default_value = "Update link feed")]
    message: String,

    /// Allow committing even when there are no changes
    #[arg(long)]
    allow_empty: bool,

    /// Do not push (only commit)
    #[arg(long)]
    no_push: bool,
}

#[derive(Args)]
struct ListArgs {
    /// Path to the feed .pb file
    #[arg(value_name = "FILE", default_value = "feed/mylinks.pb")]
    file: PathBuf,

    /// Show detailed, multi-line output
    #[arg(short = 'l', long = "long", alias = "detail")]
    long: bool,

    /// Filter by Tags (comma separated values)
    #[arg(short, long, value_parser = parse_tags)]
    tags: Option<Vec<String>>,

    /// Filter by Date (YYYY-MM-DD)
    #[arg(short, long, value_name = "YYYY-MM-DD", value_parser = parse_date)]
    date: Option<Date>,
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
    id: Option<Uuid>,
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
    // Enable with env vars: RUST_LOG=info (works because we use EnvFilter)
    #[cfg(feature = "logs")]
    {
        use tracing_subscriber::{EnvFilter, fmt};
        let _ = fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init(); // ignore "already set" in tests
    }
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
        Commands::List(args) => cmd_list(args.file, args.long, args.tags, args.date),
        Commands::Html(args) => cmd_html(args.file, args.out, args.title),
        Commands::Publish(args) => cmd_publish(
            args.file,
            &args.remote,
            args.branch,
            &args.message,
            args.allow_empty,
            args.no_push,
        ),
    }
}
