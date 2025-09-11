<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="logo-monochrome.svg">
    <img alt="Linkleaf" src="logo.svg" width="420">
  </picture>
</p>

<h1 align="center">Linkleaf</h1>

<p align="center">
  Manage <strong>protobuf-only</strong> Linkleaf feeds (<code>linkleaf.v1</code>)
</p>

<p align="center">
  <a href="https://github.com/doriancodes/linkleaf/actions/workflows/ci.yml">
    <img alt="CI" src="https://github.com/doriancodes/linkleaf/actions/workflows/ci.yml/badge.svg">
  </a>
</p>

---

This repository contains the `linkleaf-cli` and `linkleaf-core` code.

> ⚠️ **Warning**: both the CLI and the public lib crate are under active development; details may change between versions.

## Feed and Link Schemas

Defined in [`feed.proto`](crates/linkleaf-core/proto/linkleaf/v1/feed.proto):

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

## Roadmap / Ideas

- Export feeds to JSON, Markdown.
- Import from bookmark managers or RSS.
- Create Atom/RSS feed from feed
- Signatures for feeds

### Next release
- Populate name from link (automatic, feature?)
