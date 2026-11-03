# Production Verification

## Frontend presentation rollout — 2026-10-05 (later)

Promotion of `master` (`0a3d52a`) carrying the client-side presentation layer: every view now renders human-readable states, field labels and values (shared `client/src/catalog/presentation.ts` plus `StatePill.vue`) instead of raw enums, field keys, JSON or internal timestamps. No server code changed; the worker stays on `347cfc4`.

- Frontend: Vercel auto-deployed `master` to production. `https://mixsets.syco23.org` returns 200 and serves the rebuilt bundle (`index-UzwuGr0X.js`) that contains the new presentation strings (`Archive ID`, `Needs a curator`, `Awaiting your review`, `Finds & fills`, `No login needed`, `Confirmed by sources`, `NO GAPS`).
- Backend: unchanged and healthy — `https://mixsets-api.syco23.org/api/live` returns `{"ok":true,"service":"syco23-mixsets"}`; container `syco23-mixsets-api:347cfc458ca9` reports healthy.
- Verification before promotion: `pnpm typecheck`, `pnpm test` (7 domain + 57 client + 119 server + 13 deployment) and `pnpm build` all pass; the local Playwright e2e suite is 12/12.

## Recorded rollout — 2026-10-05 (later)

Promotion of `master` (`a0116076a9ab071f422582bd14df4839596e0a95`) carrying the token-free Mixcloud provider and local filename auto-tagging.

- Backend: release worktree `/opt/syco23-releases/mixsets-a011607`, image `syco23-mixsets-api:a0116076a9ab`; container **healthy**. Pre-deploy verified backup `/opt/syco23-mixsets-backups/catalog-a011607.sqlite`.
- Public TLS `https://mixsets-api.syco23.org/api/*`: `live`, `auth/session`, all five catalog indexes and `catalog/review` return 200; session `{"authenticated":true,"mode":"off","authDisabled":true}`. Data intact through the recreate (4 mixes, 7 provider sources, 21 claims, 1 review item).
- Live provider health now includes a credential-free **mixcloud `ready`** alongside freeteknomusic/archiveorg/youtube/hearthis/discogs `ready` and soundcloud `limited`.
- Frontend: Vercel auto-deployed `master`; a deployed-browser check on `https://mixsets.syco23.org` shows the Mixcloud card on `/providers` and the **Auto-tag from filenames** control on `/local-tags`, with no console/page errors.
- Rollback retained: previous release `/opt/syco23-releases/mixsets-582aea7` and image `syco23-mixsets-api:582aea76f665`.

## Production import and controlled conflict — 2026-10-05

The final Task 9 item was exercised against the live catalog (`a011607`).

- **Mackitek discovery** — `POST /api/jobs {provider:'youtube', query:{q:'Mackitek'}}` returned 6 long-form results; the first, "01h20 dans le rouge" (80.6 min), was imported.
- **Import** — created mix `mix_5fc3b9e4-bc5d-4f0a-b514-ef65e1475d61` (source `youtube:video:1hjkYjJR36A`); a parsed cover claim was correctly held in Review rather than auto-selected.
- **Enrichment** — attempted soundcloud/archiveorg/youtube/hearthis/mixcloud, applied 1; soundcloud reported `limited`; remaining gaps: `recordingDate`, `genres`, `styles`, `artists`, `crews`, `labels`, `events`.
- **Independent-session verification** — an unauthenticated request (no cookie, no Origin) read the record (HTTP 200) and it appears in the public mix index.
- **Controlled conflicting claim** — a real second source, Mixcloud "Mackitek Records - Dans Le ROUGE" (4,832 s vs. the YouTube copy's 4,838 s — the same recording), was imported as `mix_2489ae19…` and merged as the duplicate. The survivor kept its selected `durationMs` 4,838,000 and title; the duplicate's 4,832,000 became a field-level Review item; both provider identities followed the survivor and the retired id still resolves (HTTP 200).
- **Cleanup** — every Review item was rejected (survivor values retained) and the duplicate was retired by the merge, so no disposable record remains. The only lasting change is the one real imported mix: the public index lists `total 5`, all `reviewState ready`, and `/api/catalog/review` is empty.

A latent merge defect was found and fixed during this exercise: a conflicting merge passed the duplicate's stored provider source (carrying `addedAt`) where a bare `ProviderRef` is required, failing strict validation. Fixed in `347cfc4` ("fix(catalog): keep a dated provider source out of the merge conflict claim") with a regression test; the deployed worker and the live exercise both run the fixed build.

## Recorded rollout — 2026-10-05

Promotion of `master` (`582aea76f665e9ced494670742560890f08d767c`, released as image `syco23-mixsets-api:582aea76f665`) beyond the previously deployed `eded9b3`, plus the frontend promotion and a deployed-browser click-through. Performed over SSH to VPS-L.

Backend:
- Release worktree `/opt/syco23-releases/mixsets-582aea7` (detached at `582aea7`); private `deploy/vps/.env` carried over; `/opt/syco23-mixsets-current` re-pointed to it and `syco23-mixsets-current-release.json` updated.
- Container `syco23-mixsets-api` recreated on `syco23-mixsets-api:582aea76f665`; `docker ps` reports **healthy**.
- Pre-deploy verified backup `/opt/syco23-mixsets-backups/catalog-582aea7.sqlite` (319,488 bytes; integrity `ok`, 0 FK violations, 24 tables, `mixes 4`, `provider_sources 7`, `field_claims 21`, `review_items 1`). Restore proof to a new path passed (`restore-proof-582aea7.sqlite`, `mixes 4`).
- Named volume `syco23-mixsets_mixsets_data` kept; the same data survived the recreate (4 mixes, 7 sources, 21 claims, 1 review item).
- Public TLS `https://mixsets-api.syco23.org/api/*`: `live`, `auth/session`, `catalog/mix`, `catalog/artist`, `catalog/crew`, `catalog/label`, `catalog/event` and `catalog/review` all return HTTP 200. Session without cookies: `{"authenticated":true,"mode":"off","authDisabled":true}`.
- Rollback retained: prior image `syco23-mixsets-api:eded9b3006c6` and release `/opt/syco23-releases/mixsets-eded9b3`.

Frontend:
- The Vercel project `syco23-mixsets` auto-deploys the `master` branch to production; the current production deployment is **Ready** and aliased to `https://mixsets.syco23.org` (and `https://syco23-mixsets.vercel.app`). The built bundle carries the API base `https://mixsets-api.syco23.org`.

Deployed-browser click-through (headless Chromium 153, real `https://mixsets.syco23.org`):
- Desktop (1440×1000) and mobile (390×844): title `SYCO23 Mixsets`, "Mixes" heading present, 4 mix links, a mix detail opened, and `/artists`, `/crews`, `/labels`, `/events`, `/review` and `/providers` all rendered. No horizontal overflow and **no console/page errors** on either viewport.

Limits: provider-wide live enrichment verification (the Kan10 / Mackitek recordings) is not part of this record. The backup note above corrects an initial attempt that used a read-only connection and therefore missed un-checkpointed WAL content; the verified snapshot was re-taken over a read-write connection. The backend rollback path is the retained `eded9b3` image + release. The fixed `backup_catalog.py` / `catalog_bundle.py` (snapshot via `VACUUM INTO` over a read-write connection) are installed on this release; a tooling run against the live database produced a self-contained 24-table snapshot.

## Live enrichment verification — 2026-10-05

Run against production providers with the worker's private credentials, in a **disposable** database via a one-off container (`--env-file deploy/vps/.env`, temporary `CATALOG_DB_PATH`); the production catalog was not mutated.

Provider health (live): freeteknomusic `ready`, archiveorg `ready`, **youtube `ready`** (Data API key accepted), hearthis `ready`, **discogs `ready`** (artist/label enrichment authenticated), **soundcloud `limited`** ("Source artwork lookup available; search requires SoundCloud app credentials") — SoundCloud-wide search remains unavailable.

- **Mackitek search** — live YouTube search returned qualifying long-form results (e.g. "MackiTek 3672 CD 01 (FULL ALBUM)", "SET MACKITEK TRIBE DU SUD"; confidence 0.28–0.46).
- **Kan10 recording cover** — importing the known Kan10 video (`fuGyMOcQZgg`) with no cover, then enriching, **automatically filled the cover** `https://i.ytimg.com/vi/fuGyMOcQZgg/hqdefault.jpg` (claim evidence `direct`, "Refreshed established provider resource identity directly"). The same cover is already selected on the live Kan10 mix.
- **Discogs entity lookup** — live lookups succeed: `Kan10` → artist `724857` (profile + image) and `Mackitek Records` → label `80539` (image). The recording's performer entities were **not** auto-confirmed (a name alone is not proof), so `artists`/`crews` stay missing until a curator links them — the intended behavior.
- Remaining missing fields on the disposable record: `recordingDate`, `styles`, `artists`, `crews`, `labels`, `events`.

Not covered here: importing a Mackitek result into the *production* catalog and confirming its shared detail from an independent session, and the controlled conflicting-claim/Review exercise. There is no disposable-record delete route, so production was deliberately left untouched.

## Local browser click-through — 2026-10-05

Automated end-to-end click-through was run on `master` (`bdf328a`) with Playwright 1.63.0 driving headless Chromium 153 (`chromium-1243`) against a real Vite dev server, the real catalog API, real curator sessions and a real SQLite catalog; only the external provider boundary is a deterministic fixture. Command: `pnpm test:e2e`. Result: **10/10 passed (57.6s)**.

The three flows previously recorded below as "not a pass" now have running-browser coverage:

- **Duplicate-merge comparison view** — previews survivor and duplicate with both revisions, confirms the merge, and the retired id still resolves.
- **Field-evidence panel** — a fresh public browser context sees a committed curator value *and* its `field-evidence` panel, with the editor hidden.
- **Inbound-mix list on an entity page** — the entity page lists the second mix that references it and links to it.

Also covered in the same run: all five public indexes and shared detail links, review-rejection persistence after reload, event month-precision round-trip, import → enrich → shared reload, narrow (390 px) navigation without horizontal overflow, browser migration retry retaining originals, and stale-save revision conflict.

Limit: this is an **automated headless-Chromium** click-through on the local revision, not a manual click-through in a visible browser and not against the deployed worker. Production frontend promotion and a deployed-browser click-through remain open (see the rollout below).

## Recorded rollout — 2026-10-04

This evidence describes the deployed `eded9b3` revision. Local `master` contains later catalog and branch-integration work and has not been verified as the live revision.

Backend deployed to IONOS VPS-L through Remote Desktop Commander. Runtime code revision: `eded9b3006c600a35b3b0f09573e110500e63380`; image: `syco23-mixsets-api:eded9b3006c6`. The release is at `/opt/syco23-releases/mixsets-eded9b3`, with `/opt/syco23-mixsets-current` pointing to it. The service is running and Docker reports **healthy**.

The user selected **App-Login aus**. Vercel project `syco23-mixsets` has `AUTH_MODE=off` and `VITE_AUTH_MODE=off` for production, preview and development. The backend mode was retrieved from Vercel and copied with `deploy/vps/sync_auth_mode.py` into the private worker env before container creation. This is deployment-time synchronization; later Vercel env edits require another sync/recreation and frontend rebuild. Vercel deployment protection was not changed.

Fresh evidence:
- `pnpm typecheck`, `pnpm test` and `pnpm build`: passed. **112 tests**: 5 domain, 24 client, 77 server, 6 deployment.
- Production Docker build passed; the runtime now installs the same pinned pnpm version as the build stage. Build context excludes private env files, dependencies, generated output and databases.
- Public TLS `/api/live`, `/api/auth/session`, all five catalog index routes and `/api/catalog/review`: HTTP 200.
- Session response without cookies: `{"authenticated":true,"mode":"off"}`.
- Anonymous POSTs from the preview origin reach input validation (HTTP 400 for deliberately invalid import/job payloads); unapproved-origin import is refused (403). No user records were created by these checks.
- Exact-origin CORS checked for the newly built Vercel preview. The real mix index route is `/api/catalog/mix`, singular.
- Named volume `syco23-mixsets_mixsets_data` is mounted read/write at `/app/data`. A separate SQLite probe was written, read successfully after force-recreating the container, then removed.
- Online catalog backup and restore-to-new-file both passed SQLite integrity and foreign-key validation. Backup: `/opt/syco23-mixsets-backups/catalog-eded9b3.sqlite`.
- Prior image and private deployment config retained at `/opt/syco23-mixsets-rollback/20261004`.
- Vercel preview at the runtime code revision is **READY**: https://syco23-mixsets-58y0s0m1v-system-corrupt.vercel.app . Homepage and generated JS return 200; the bundle contains the configured API URL and open-access UI. Browser click-through was not performed in this rollout; a later local automated click-through on `master` is recorded above.

The shared catalog starts empty. Existing browser-local libraries were not accessed or silently imported; the archive access screen retains the explicit migration action. Discovery/waveform queues remain in memory and require one worker. Production frontend promotion and merging [PR #2](https://github.com/murphieslaw23/onetagger/pull/2) were not performed as part of this backend deployment.

## Earlier verification record

The following record predates this rollout; its deployment blockers are superseded by the current rollout above.

Date: 2026-10-04 (local checks re-run; browser and VPS findings unchanged from 2026-10-03)

## Local implementation checks

- `pnpm typecheck`: passed for shared domain, Vue client and Node server.
- `pnpm test`: passed, 152 tests total (7 domain, 42 client and 103 server). Coverage includes schema validation, provider/resource identity, persistence/reopen, claims/review revisions, curator sessions, CORS, import retries, migration, waveform media, SQLite backup/restore, provider selection, HTTP validation edges and local MP3 tag writing.
- `pnpm test:deployment`: passed, 8 tests covering the VPS deployment package.
- `pnpm build`: passed for client and server. Rollup reports removable `@__PURE__` comment-position warnings from Zod; output is produced successfully.

Coverage now includes, beyond the earlier schema / provider / migration / media / backup set:

- **Catalog error classification** — reachable domain failures keep their HTTP status (404 missing record, 409 revision conflict, 400 bad request), while an internal fault returns 500 with a generic body and never echoes paths, SQL or secrets.
- **Durable enrichment runs** — a run row is written before provider work starts, so a crashed pass is recorded `interrupted` and stays retryable. Worker start closes runs left `running` by a previous process. A provider failure still completes the run, so valid results from other providers are not discarded.
- **Duplicate merge** — source identities, claims, review decisions, media and relationships move to the survivor. A field disagreement becomes a Review item rather than being resolved by write order. The retired id keeps resolving, and a stale second curator session is refused.
- **Field evidence** — the claim behind a selected value is retrievable and public wherever the record itself is public; a proposed record's evidence stays hidden.
- **Inbound entity navigation** — an entity page lists the mixes that reference it, including mixes that arrived through import or enrichment rather than legacy migration.
- **Browser-local state** — `useMixStore` holds provider health and job progress only, persists nothing, and cannot make an offline or failed write look committed.
- **Provider fan-out bounds** — the discovery queue caps concurrency and answers 429 instead of accepting unbounded work.
- **Search normalization** — `normalizeQuery` preserves `live`, `set` and `dj`, because those distinguish one recording from another ("Live at Wacken", "DJ Koalisson"). Only the `mix` family is dropped, and only in trailing position.

Smoke tests below were run on 2026-10-03 and were **not repeated** after the 2026-10-04 changes:

- Browser smoke test: local public catalog, curator login, Archive.org discovery and the known public Kan10/Mackitek YouTube video resolved. A low-confidence Archive.org result was not imported. The selected YouTube source created a shared detail and provider claims; the matching cover filled, alternative provider values remained in Review, and the interface reported remaining fields and provider errors.
- Local tagger smoke test: `/local-tags` loaded on the fresh Vite server, reported six provider states, required curator login for enrichment, and confirmed browser folder-picker support. No real user folder was selected; MP3 read/write is covered by unit tests.
- Desktop and 320 CSS-pixel mobile checks: no horizontal overflow at tested widths. A 312 CSS-pixel emulation is below the app's existing 320-pixel minimum and overflows by 8 pixels.

**Resolved 2026-10-05:** the duplicate-merge comparison view, the field-evidence panel and the inbound-mix list on an entity page previously had unit/HTTP-level coverage only. They now have automated running-browser click-through coverage on `master` (local 10/10 e2e and the 2026-10-05 deployed-browser record at the top of this document).

## Production status

Deployment was requested and the VPS was checked read-only through `ssh ionos_vps_l`. At verification time the API was healthy at commit `7368973`, with Discogs ready. **No deployment has been performed.** The deployed commit is still `7368973`, so the catalog work committed after it is not live.

Before rollout, set `CURATOR_PASSWORD_HASH` privately with `deploy/vps/setup_curator.py`, deploy the pinned Node 22.23.3 image and persistent `/app/data` volume, verify public reads/authenticated writes/CORS, perform and verify an online backup, deploy the frontend, migrate each real browser library, and test after worker recreation. The discovery job queue is still in-memory and requires a single worker; waveform analysis runs are persisted per mix in `analysis_runs` and interrupted runs are surfaced rather than lost.

Unchanged blocking prerequisites:

- the private VPS `.env` has no `CURATOR_PASSWORD_HASH`;
- the current API container has no mounted data volume;
- the proposed Compose configuration requires the curator hash and introduces the persistent catalog volume.

Replacing the live worker before privately configuring the curator and confirming the catalog and backup plan would be unsafe. The Vercel frontend was not deployed.

Before rollout: set `CURATOR_PASSWORD_HASH` privately with `deploy/vps/setup_curator.py`, deploy the pinned Node 22.23.3 image and the persistent `/app/data` volume, verify public reads / authenticated writes / exact-origin CORS, perform and verify an online backup, deploy the frontend, migrate each real browser library, and re-check after worker recreation. The discovery and waveform job queues remain in memory and require a single worker.

Provider constraints remain: SoundCloud official search needs its client secret or a supported access token; YouTube text search needs its API key. A provider failure leaves fields missing and is reported as such — it is never presented as "nothing new to add".
