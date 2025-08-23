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

---

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

```bash
linkleaf init <FILE> [--title <TITLE>] [--version <N>]
linkleaf add  <FILE> --title <TITLE> --url <URL> [--summary <S>] [--tags <CSV>] [--via <URL>] [--id <ID>]
linkleaf list <FILE>
linkleaf print <FILE>
linkleaf html <FILE> --out <HTML> [--title <PAGE_TITLE>]
```

### Initialize a feed
```bash
linkleaf init
```
Creates a new feed file with optional title and version.

### Add a link
```bash
linkleaf add --title "The Rust Book" \
  --url "https://doc.rust-lang.org/book/" \
  -g rust,learning,book \
  --via "https://rust-lang.org"

linkleaf --title "Tokio - Asynchronous Rust" \
  --url "https://tokio.rs/"
```

If no --id is given, one is derived automatically.

### List links (short view)

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

### List links (detailed view)
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

```bash
linkleaf html feed/mylinks.pb --out assets/index.html --title "My Links"
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
