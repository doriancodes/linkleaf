<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="logo-monochrome.svg">
    <img alt="Linkleaf" src="logo.svg" width="420">
  </picture>
</p>

<h1 align="center">Linkleaf</h1>

<p align="center">
  Manage <strong>protobuf-only</strong> Linkleaf feeds (<code>linkleaf.v1</code>) within your terminal.
</p>

<p align="center">
  <a href="https://github.com/doriancodes/linkleaf/actions/workflows/ci.yml">
    <img alt="CI" src="https://github.com/doriancodes/linkleaf/actions/workflows/ci.yml/badge.svg">
  </a>
</p>

---

`linkleaf` is a simple, protobuf-backed feed manager for storing and organizing links.
It uses a Protocol Buffers schema to define a portable, versioned format for feeds and individual link entries.

The command-line interface (`linkleaf`) lets you create a feed, add links and list/inspect entries from a compact `.pb` file.

> ⚠️ **Warning**: both the CLI and the public API are under active development; details may change between versions.


## Features

- **Portable format** — uses protobuf messages (`Feed`, `Link`).
- **Minimal metadata** — title, URL, datetime, tags, optional summary/referrer.
- **Stable IDs** — default ID is a **UUID v4** (you can override with `--id`, which must be a valid UUID).
- **Local-first** — single binary `.pb` file; no server required.
- **Filtering** — list by tags and/or by date (day-precision).

## Usage

```bash
linkleaf init    [FILE] [--title <TITLE>] [--version <N>]

linkleaf add     [FILE]
                 --title <TITLE> --url <URL>
                 [--summary <S>] [--tags <CSV>] [--via <URL>] [--id <UUID>]

linkleaf list    [FILE]
                 [-l|--long]
                 [--tags <CSV>]
                 [--date <YYYY-MM-DD>]

```

## Defaults
- `FILE` defaults to `feed/mylinks.pb` for all commands.
- `add` uses today’s date automatically (`YYYY-MM-DD`, UTC).
- `--tags` is comma-separated (e.g., `rust,book,learning`).
- `add --id` **must be a valid UUID** (v4 recommended). Without `--id`, a new UUID is generated.
- `add` sets the link’s datetime to local time in the format `YYYY-MM-DD HH:MM:SS`.
- Re-adding a link updates an existing entry when the **ID matches** or the **URL matches**; the updated link moves to the front.
- `list --date` accepts `YYYY-MM-DD` and matches links on that calendar day (local time).
- Links are kept newest-first (inserts and updates end up at index `0`)

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
linkleaf add --title "Tokio - Asynchronous Rust" \
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
  --id 123e4567-e89b-12d3-a456-426614174000
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

  1. Thu, 2 Oct 2025 02:49:07 +0000  Serde
     https://serde.rs/
  2. Thu, 2 Oct 2025 02:49:02 +0000  Tokio — Async Rust [rust,async,tokio]
     https://tokio.rs/
```
Detailed (multi-line) view:

```bash
linkleaf list -l
```
Example output:

```bash
Feed: 'My Links' (v1)

- [Thu, 2 Oct 2025 02:49:07 +0000] Serde
id: aa4f1396-7a78-410d-83cd-acaf0b0fbe4e
url: https://serde.rs/

- [Thu, 2 Oct 2025 02:49:02 +0000] Tokio — Async Rust
id: b3a20989-0ba1-4c4e-80d4-bd2cfa33e9a9
url: https://tokio.rs/
via: https://github.com/tokio-rs
summary: The async runtime for Rust
tags: rust,async,tokio
```
Filter by tags or by date (day-precision):
```bash
# Any of the listed tags will match (case-insensitive)
linkleaf list --tags rust,book

# Only links from that calendar day (local time)
linkleaf list --date 2025-08-23
```

### Generate RSS feed
```bash
# Generates the rss from feed/mylinks.pb with the site title "My Links" and site link "https://www.example.com"
linkleaf gen-rss

# Generates the rss from feed/mylinks.pb with the site title "Awesome Links" and site link "https://www.example.com"
linkleaf gen-rss -t "Awesome Links"

# Generates the rss from feed/mylinks.pb with the site title "My Links" and site link "https://www.yourwebsite.com"
linkleaf gen-rss -l "https://www.yourwebsite.com"
```

## Feed Schema

Defined in `proto/linkleaf/v1/feed.proto`:

- Link
  - `id` (string) — auto-derived if omitted
  - `title` (string, required)
  - `url` (string, required)
  - `date` (string, YYYY-MM-DD)
  - `summary` (optional string)
  - `tags` (optional repeated strings)
  - `via` (optional string)

- Feed
  - `title` (string)
  - `version` (uint32)
  - `links` (repeated Link, newest first)

## Development

Protobufs are compiled at build time via prost-build.

To recompile after changing the .proto schema:

```bash
cargo clean
cargo build
```

# Fixes
- Add link gives a default name if none is given when the feed is created
