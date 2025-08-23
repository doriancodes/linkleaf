# linkleaf-rs


linkleaf-rs is a simple, protobuf-backed feed manager for storing and organizing links.
It uses a Protocol Buffers schema to define a portable, versioned format for feeds and individual link entries.

The command-line interface (linkleaf) lets you create a feed, add links, and inspect your stored data — all while persisting to a compact .pb file.

## Features
- Portable format — uses protobuf messages (Feed, Link) for long-term stability.
- Add links with title, URL, date, tags, summary, and optional referrer.
- Deterministic IDs — default ID derived from sha256(url|date)[:12].
- Local storage — feeds saved as a single binary .pb file.
- CLI commands for initialization, adding entries, listing, and pretty-printing.

## Usage
### Initialize a feed

```bash
linkleaf init --file mylinks.pb --title "My Links" --version 1
```
Creates a new feed file (mylinks.pb) with optional title and version.

### Add a link
```bash
linkleaf add --file mylinks.pb \
  --title "Rust Book" \
  --url "https://doc.rust-lang.org/book/" \
  --date 2025-08-23 \
  --summary "The official Rust programming language book" \
  --tags rust,learning,book \
  --via "https://rust-lang.org"
```

If no --id is given, one is derived automatically.

### List links (short view)

```bash
linkleaf list --file mylinks.pb
```
Example output:

```bash
Feed: 'My Links' (v1) — 1 links

  1. 2025-08-23  Rust Book [rust,learning,book]
     https://doc.rust-lang.org/book/
```

### Print links (detailed view)
```bash
linkleaf print --file mylinks.pb
```
Example output:

```bash
Feed: 'My Links' (v1)

- [2025-08-23] Rust Book
  id: 123abc456def
  url: https://doc.rust-lang.org/book/
  via: https://rust-lang.org
  summary: The official Rust programming language book
  tags: rust, learning, book

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

- Export feeds to JSON, Markdown, or HTML.
- Import from bookmark managers or RSS.
- Filtering and searching links by tags or date.
