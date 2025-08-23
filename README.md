<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="logo-monochrome.svg">
    <img alt="Linkleaf" src="logo.svg" width="420">
  </picture>
</p>

<h1 align="center">Linkleaf-rs</h1>

<p align="center">
  Manage <strong>protobuf-only</strong> Linkleaf feeds (<code>linkleaf.v1</code>) within your terminal.
</p>

<p align="center">
  <a href="https://github.com/doriancodes/linkleaf-rs/actions/workflows/ci.yml">
    <img alt="CI" src="https://github.com/doriancodes/linkleaf-rs/actions/workflows/ci.yml/badge.svg">
  </a>
</p>

---

`linkleaf-rs` is a simple, protobuf-backed feed manager for storing and organizing links.
It uses a Protocol Buffers schema to define a portable, versioned format for feeds and individual link entries.

The command-line interface (`linkleaf`) lets you create a feed, add links, list/inspect entries, render a static HTML page, and publish updates to a git remote—persisting everything to a compact `.pb` file.

## Features

- **Portable format** — uses protobuf messages (`Feed`, `Link`) for long-term stability.
- **Minimal metadata** — title, URL, date (auto), tags, optional summary/referrer.
- **Deterministic IDs** — default ID: `sha256(url|date)[:12]` (you can override with `--id`).
- **Local-first** — single binary `.pb` file; no server required.
- **HTML export** — render a minimal static page (optionally with a `style.css`).
- **Git publish** — stage, commit and push your feed to any git remote.

## Usage

```bash
linkleaf init    [FILE] [--title <TITLE>] [--version <N>]
linkleaf add     [FILE] --title <TITLE> --url <URL> [--summary <S>] [--tags <CSV>] [--via <URL>] [--id <ID>]
linkleaf list    [FILE] [-l|--long]
linkleaf html    [FILE] [--out <HTML>] [--title <PAGE_TITLE>]
linkleaf publish [FILE] [--remote <NAME>] [-b|--branch <BRANCH>] [-m|--message <MSG>] [--allow-empty] [--no-push]
```

## Defaults
- `FILE` defaults to `feed/mylinks.pb` for all commands.
- `html --out` defaults to `assets/index.html`.
- `add` uses today’s date automatically (`YYYY-MM-DD`, UTC).
- `--tags` is comma-separated (e.g., `rust,book,learning`).
- `add` **updates** an existing entry when the (derived or explicit) ID already exists.

## Examples
### Initialize a feed
Create the default feed in `feed/mylinks.pb`:
```bash
linkleaf init
```
Custom location, title and version:
```bash
linkleaf init my/links.pb --title "Reading List" --version 1
```

### Add a link
Basic add (date is auto):

```bash
linkleaf --title "Tokio - Asynchronous Rust" \
  --url "https://tokio.rs/"
```

With summary, tags and via:

```bash
linkleaf add --title "Tokio — Async Rust" \
  --url "https://tokio.rs/" \
  --summary "The async runtime for Rust" \
  -g rust,async,tokio \
  --via "https://github.com/tokio-rs"
```
If no --id is given, one is derived automatically.
Explicit ID (overrides the derived one):

```bash
linkleaf add --title "Serde" \
  --url "https://serde.rs/" \
  --id 123abc456def
```
Update an existing entry (same derived/explicit ID):

```bash
# Re-adding same URL on the same day reuses the same derived ID → updates fields
linkleaf add --title "Tokio — Asynchronous Rust" \
  --url "https://tokio.rs/" \
  --summary "Updated summary"
```
Use a non-default feed path:
```bash
linkleaf add my/links.pb --title "Prost" --url "https://docs.rs/prost"
```

### List links
Compact view:
```bash
linkleaf list
```
Example output:

```bash
Feed: 'My Links' (v1) — 2 links

  1. 2025-08-23  Tokio - Asynchronous Rust
     https://tokio.rs/
  2. 2025-08-23  The Rust Book [rust,learning,book]
     https://doc.rust-lang.org/book/
```
Detailed (multi-line) view:

```bash
linkleaf list -l
```
Example output:

```bash
Feed: 'My Links' (v1)

- [2025-08-23] Tokio - Asynchronous Rust
  id: b8f94c560b87
  url: https://tokio.rs/

- [2025-08-23] The Rust Book
  id: 04eff20db88f
  url: https://doc.rust-lang.org/book/
  via: https://rust-lang.org
  tags: rust, learning, book
```
### Render to HTML
Render the default feed to a static page:

```bash
linkleaf html
```
Render a custom feed path:
```bash
linkleaf html my/links.pb --out public/index.html
```

### Publish to a git repo
```bash
# Commit and push feed/mylinks.pb to the current upstream
linkleaf publish

# Commit and push to a specific branch (sets upstream)
linkleaf publish -b main

# Custom file path + custom message
linkleaf publish my/links.pb -m "Add two new links"

# Use a different remote
linkleaf publish --remote origin

# Commit only (don’t push)
linkleaf publish --no-push

# Allow an empty commit (e.g., to trigger CI):
linkleaf publish --allow-empty -m "chore: trigger build"
```

## Feed Schema

Defined in proto/linkleaf/v1/feed.proto:

- Link
  - id (string) — auto-derived if omitted
  - title (string, required)
  - url (string, required)
  - date (string, YYYY-MM-DD)
  - summary (optional string)
  - tags (optional repeated strings)
  - via (optional string)

- Feed
  - title (string)
  - version (uint32)
  - links (repeated Link, newest first)

## Development

Protobufs are compiled at build time via prost-build.

To recompile after changing the .proto schema:

```bash
cargo clean
cargo build
```

## Roadmap / Ideas

- Export feeds to JSON, Markdown.
- Import from bookmark managers or RSS.
- Filtering and searching links by tags or date.
