# JAV Archivist

A Windows desktop app for managing your JAV video collection. Scans folders to automatically register videos and fetches metadata from online sources to populate your library.

**Read this in other languages:** [한국어](README.ko.md) · [日本語](README.ja.md) · [中文](README.zh.md)

---

## Features

- **Library Scan** — Recursively scans specified folders and automatically registers video files
- **Metadata Scraping** — Fetches title, cast, series, maker, and tags automatically based on video code
- **Thumbnails / Sample Images** — Extracts frames using Windows Media Foundation — no external tools required
- **Sprite Seekbar** — Seekbar preview thumbnails inside the video player
- **Filter & Search** — Filter by cast, series, maker, tags, favorites, watch status, and more
- **Cast · Series · Tag Management** — Browse metadata from dedicated pages
- **Favorites / Watch History** — Track status per video
- **Logs** — View app activity in real time (enable in Settings)

---

## System Requirements

| Item | Requirement |
|------|-------------|
| OS | Windows 10 / 11 (64-bit) |
| External Programs | None (no FFmpeg needed) |
| Display | 1024 × 640 or higher |

---

## Installation

> **No release build yet?** See the [Build from Source](#build-from-source) section below.

Download the `.msi` or `.exe` installer from the Releases page and run it.

---

## Getting Started

1. **Settings → Scan Folders** — Add the folders where your videos are stored.
2. **Library → Scan** — Scan the folders to register videos.
3. **Scrape** — Select videos or run a batch scrape to fill in metadata.
4. **Browse** — Filter by cast, series, tags, and more to manage your collection.

---

## Pages

| Tab | Description |
|-----|-------------|
| Library | Browse videos in a grid view with search and filters |
| Cast | Browse registered cast members, filter videos by cast |
| Series | Group videos by series |
| Tags | Tag list and co-occurrence analysis |
| Makers | Filter videos by production company |
| Settings | Scan folders, scraping options, log settings |
| Logs | View app events in real time (requires enabling in Settings) |

---

## Build from Source

### Prerequisites

- [Node.js](https://nodejs.org/) 18 or later
- [pnpm](https://pnpm.io/) (`npm install -g pnpm`)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri prerequisites](https://tauri.app/start/prerequisites/) (includes Visual Studio C++ Build Tools)

### Build

```bash
# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Production build (output in src-tauri/target/release/bundle/)
pnpm tauri build
```

---

## Tech Stack

- **Frontend** — React 19, TypeScript, Vite, TailwindCSS, Zustand
- **Backend** — Rust, Tauri 2, SQLite (rusqlite)
- **Media Processing** — Windows Media Foundation (thumbnails & frame extraction)
- **Scraping** — rquest, scraper

---

## License

This project is for personal use.
