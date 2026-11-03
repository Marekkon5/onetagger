# SYCO23 Mixsets

A long-form DJ/live-mix archive manager derived from the provider and review ideas in [OneTagger](https://github.com/Marekkon5/onetagger), rebuilt for one-to-two-hour mixes instead of individual track tagging.

## Product model

SYCO23 Mixsets is a shared archive of long-form DJ and live mixes. Each recording has one canonical mix entry, linked to independently browsable artist, crew, label and event indexes. Provider data becomes validated, attributable claims: a proven match fills a missing field, while a conflict or an uncertain identity stays in Review until a curator decides.

The archive lives in the worker, not in the browser. The browser is a public reader and a curator control surface; it caches what it fetched and keeps a browser-local copy of your own library only until you have migrated it into the shared archive. A write that fails is never shown as committed.

Fields are allowed to stay missing. The interface distinguishes missing, disputed and unavailable information and never reports that a record is complete just because an enrichment run found nothing new.

The active web product no longer contains OneTagger's AutoTagger, QuickTag, track renamer, Spotify AudioFeatures or desktop WebView/socket flows.

## Providers

- **archive.freeteknomusic.org** — bounded HTTP directory crawler; never treated as FTP and never downloads complete audio just to discover metadata.
- **SoundCloud** — official API only; long-form discovery uses duration filtering and private app credentials. Already-linked public tracks can supply cover art through oEmbed. Audio import is limited to tracks the uploader marked downloadable.
- **Internet Archive** — Advanced Search + Metadata API.
- **Discogs** — artist / crew-like artist / label enrichment and images, plus release/master metadata import; no mix audio.
- **YouTube** — known public video metadata and thumbnails without credentials; text search with a private Data API key.
- **hearthis.at** — public track search, metadata, track artwork and provider-authorised audio without credentials, subject to provider limits.
- **Mixcloud** — public read API for long-form cloudcast search and metadata import; audio only for shows Mixcloud reports as downloadable.
- **MusicBrainz** — public recording/release/release-group metadata with a descriptive User-Agent; no mix audio.

See [docs/PROVIDERS.md](docs/PROVIDERS.md) for the per-provider import capability matrix.

Import rights are deployment-wide: `IMPORT_GLOBAL_RIGHTS_CONFIRMATION` defaults to `true`, so the operator confirms once in private deployment configuration and the server records that attestation for every import. There is no per-mixset rights prompt. Providers that restrict a specific track or show still fail honestly; no protected stream is bypassed.

The Providers screen links to setup guides for SoundCloud, Discogs and YouTube. Credentials are entered only in an interactive VPS terminal, never in the public web app. A known public SoundCloud, YouTube, hearthis.at or Mixcloud link can be previewed as artwork from a mix detail view. Direct public Freeteknomusic and Archive.org audio can be analyzed into a real waveform on demand.

## Local development

```bash
pnpm install
cp .env.example .env
python3 deploy/vps/setup_curator.py .env
pnpm dev:api
# second terminal
pnpm dev:web
```

Web: http://localhost:5173

API: http://localhost:8787

The local API reads `.env` at startup. Set `CURATOR_PASSWORD_HASH` with a private password hash before using catalog write routes; generate it interactively with `python3 deploy/vps/setup_curator.py .env`. The setup routine never prints the password or hash.

The shared catalog starts empty. Public provider metadata is fetched by the worker; curator login is required for imports, edits, enrichment and review decisions.

## Browsing and curating

- `/` lists mixes; `/artists`, `/crews`, `/labels` and `/events` list the other indexes. Each supports search and pagination.
- A mix detail links to its performers, crews, labels and events. An artist, crew or label page lists the mixes that reference it, and its sourced fields with the claim behind each selected value.
- **Enrich missing fields** runs every usable provider capability against one record and reports what was applied, what agreed, what needs review and which fields remain missing. Runs are recorded: an interrupted run is visible and retryable.
- **Review** shows field conflicts and uncertain identities across all index types, with the current and proposed value, the source link and why the match was proposed.
- **Merge duplicates** compares two confirmed records side by side before confirming. Fields both records state differently stay in Review instead of being resolved by merge order, and the duplicate's old link keeps working.

## Local file tagging

`/local-tags` scans a folder of MP3s entirely in the browser; originals are never changed.

- **Auto-tag from filenames** derives artist, title and year from each name and scores how much the filename actually establishes. Files reaching the 80% evidence rate are accepted automatically; weaker names are left for a curator.
- Accepted files are **enriched automatically** — only missing fields are added, and existing tags and covers always win — using the selected providers.
- **Optional library sync** uploads accepted records that carry a public provider source to the shared archive, so a local file becomes a shared record. A file with no provider source is reported as not synced rather than imported under a private path.
- **Write tagged copies** writes normalized ID3 tags and front covers into a separate `Mixsets-tagged` folder, leaving the originals untouched.

## Validation

```bash
pnpm typecheck
pnpm test
pnpm build
```

`pnpm test` runs the shared-domain, client API/store, server, import-worker and deployment suites. The SQLite Node API emits Node's experimental SQLite warning on Node 22; production is pinned to Node 22.23.3. Durable import jobs stay disabled unless `IMPORTS_ENABLED` is set, curator authentication (`AUTH_MODE=on` with a real hash) is configured, and `IMPORT_WORKER_SECRET` is present. Route-level behavior is covered by `pnpm test:e2e`.

## Verification status

The production record documents a healthy IONOS VPS-L rollout with persistent catalog storage and the user-selected `AUTH_MODE=off`. The worker runs the latest server code (`347cfc4`); the Vercel frontend auto-deploys `master` to `https://mixsets.syco23.org`. See [docs/PRODUCTION_VERIFICATION.md](docs/PRODUCTION_VERIFICATION.md) for the dated runtime evidence and remaining limits.

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/MIGRATION_FROM_ONETAGGER.md](docs/MIGRATION_FROM_ONETAGGER.md).

## License and attribution

This repository is a fork of OneTagger by Marekkon5 and contributors and retains the upstream **GPL-3.0** license. The SYCO23 branch replaces the active desktop/track product with a web-focused long-form mix manager while preserving attribution and the fork's license obligations.
