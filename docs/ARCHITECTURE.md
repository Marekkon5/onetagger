# Architecture

The browser is a public archive and curator control surface. The Node worker owns provider credentials, canonical catalog records, field evidence, review decisions and generated media.

```text
Vue 3 / Quasar
  -> shared Zod domain contracts
  -> credentialed catalog API
       -> public paginated mix / artist / crew / label / event indexes
       -> per-record field evidence and inbound mix links
       -> curator login, import, edit, enrich, merge and review

Node 22.23.3 worker
  -> provider registry and existing discovery adapters
  -> identity resolution and conservative field-claim merger
  -> SQLite repository (one VPS worker, WAL, foreign keys)
  -> durable enrichment-run log (migration 006)
  -> persistent media directory (validated waveform PNGs)
  -> in-memory discovery and waveform job queues (bounded concurrency)
  -> SQLite-backed waveform analysis runs (migration 008)
  -> curator-gated discovery, provider health, artwork and direct-waveform routes
  -> optional import control plane (migration 010, HMAC worker, private artifacts)
```

## Persistence boundaries

- `packages/domain` owns strict canonical/provider/import schemas, normalization, field validation and shared missing-field definitions.
- `server/src/catalog/repository.ts` owns records, provider/resource identities, typed relationships, aliases, claims, decisions, sessions, migration batches, media references, durable enrichment and analysis runs, and the normalized read projection (`mixes`, `events`, `entity_details`, `entity_aliases`, `record_terms`, `record_links`, `record_assets`, `selected_evidence`) which is re-synced on every commit.
- Schema changes are ordered SQL files in `server/src/catalog/migrations/`, applied by `PRAGMA user_version`. Adding one requires both the `.sql` file and an entry in the array in `database.ts`.
- Network requests and audio decoding happen before short synchronous SQLite transactions.
- SQLite uses a persistent Docker volume at `/app/data`; online backups are verified before restore, and restore targets must be new paths.
- Browser state is a cache only. A mutation updates UI state after the API returns its committed canonical record. The app never deletes the browser's legacy `syco23.mixsets.library` key: the curator can dismiss or retry migration and the local copy is only ever read.

## Identity and evidence

1. A provider identity is `(provider, resourceType, externalId)`; equal IDs in different namespaces remain distinct.
2. Repeated source imports are idempotent. Names alone never merge artists or events.
3. Direct claims automatically fill gaps only for linked provider identities and confirmed records. Parsed or uncertain matches stay in Review.
4. Conflicts preserve selected fields. Decisions carry the current record revision; stale items must be refreshed.
5. Cover, artist portrait, crew/label logos, event flyers and waveforms are separate asset roles.
6. Recording, upload and event dates remain separate; upload dates and channel names are not recording facts or performers.
7. A record stores `selectedEvidence` as field → claim id. The claim itself is retrieved separately, so a reader can see which source established a value and why a competing claim was rejected.
8. Inbound relations are derived on read, not from stored back-references. An entity's connected mixes are computed from the mix records, because a mix that arrived through import or enrichment never wrote a back-pointer.

## Client presentation layer

The browser maps every machine identifier to plain wording through `client/src/catalog/presentation.ts` instead of rendering raw enums, field keys, JSON or internal timestamps. It owns:

- `fieldLabel` — shared labels for the missing-fields strip, the editor, field evidence and Review, so keys like `durationMs`, `cover` and `possibleDuplicate` are never shown to a reader.
- `providerLabel` and the record-kind, verification, review-state, job/run/analysis-state, health, evidence and disposition label maps.
- `humanValue` — renders dates with their precision, shortens URLs to their origin and never falls back to `JSON.stringify` for object values.
- `formatTimestamp` / `timeAgo` — human timestamps for run and claim history.
- `statePillLabel` / `stateTone` — consumed by the shared `StatePill.vue` so every state uses one consistent wording and colour.

This keeps the data model machine-readable while the interface stays readable for non-technical curators and public readers.

## Duplicate merge

A curator can merge two confirmed duplicates transactionally. The survivor keeps its selected evidence and receives the duplicate's source identities, claims, review decisions, media and relationships. Where both records hold different values for the same single-valued field, the survivor's value stays selected and the duplicate's value becomes a Review item — a disagreement is never resolved by merge order. The retired id becomes a legacy alias so detail links that were already shared keep resolving, and both revisions are checked so a second curator session cannot merge over newer curation.

## Enrichment runs

Every enrichment pass writes a run row before provider work starts, so a crash leaves a visible `interrupted` run instead of a record that is implicitly "enriching" with no trace. A partial unique index keeps at most one unfinished run per record, and worker start closes runs left `running` by a previous process. A provider failure is not a run failure: the run completes with its errors recorded, so valid results from other providers survive.

## Access and operations

Public readers can list and open confirmed records, read field evidence and follow entity/event links. Proposed identity details are hidden from public index and detail routes. Curator writes require a scrypt password hash, rate-limited login, hashed server-side session tokens, an HTTP-only cookie, and an exact configured origin. Outside production `localhost:5173` and `127.0.0.1:5173` are added to the configured origin list; any other origin (for example the Playwright dev server on port 15173) must be listed in `CORS_ORIGIN` explicitly. Production has no implicit origin.

Domain errors are mapped to HTTP status by an explicit allow-list of message prefixes rather than a broad pattern, so an unrecognized internal fault becomes a generic 500 instead of leaking a path or SQL fragment in a 400 body.

Discovery jobs and waveform jobs remain in memory and are scoped to the single running worker. Restarting the worker preserves catalog data, claims, decisions, media and enrichment history, but active discovery/waveform jobs are not durable. Multi-worker catalog deployment is not supported until those queues are persisted.

## Import control plane

Durable metadata/audio imports are a separate, opt-in plane. They stay dark (`503`) unless `IMPORTS_ENABLED` is set **and** a real curator password hash is configured. Curator sessions own `/api/imports`; the import worker authenticates to `/internal/imports` with HMAC-SHA256 over timestamp, method, path and body.

**Current gap:** with `AUTH_MODE=off`, every request authenticates as the curator (`server/src/auth/curator.ts`), so a deployment that enables imports with a configured hash still exposes creation/finalize anonymously. Do not combine `AUTH_MODE=off` with an enabled import plane until this is fixed; see `docs/AUDIT.md` H4.

Provider policy is shared domain code: SoundCloud/YouTube/hearthis expose metadata only; Freeteknomusic and Archive.org may acquire audio when an explicit non-metadata rights basis is attested. User uploads store a private `urn:syco23:upload:…` handle, never a filesystem path. The worker never opens SQLite: it leases jobs, writes private artifacts, scores evidence and reports completion. Non-retryable failures (blocked URLs, rejected media, policy) stay `failed`; retryable failures re-queue with backoff.

A single-user local mode exists for the operator's own machine. Setting `CURATOR_AUTH_DISABLED=true` makes every request authenticate as the curator, so the GUI never blocks on a login. It is opt-in, is ignored whenever `NODE_ENV=production`, and logs a warning on start. The deployed worker sets `NODE_ENV: production` and requires `CURATOR_PASSWORD_HASH`, so it cannot be opened by this variable. The exact-origin check and all input validation continue to apply in this mode; only authentication is skipped.

Restarting the worker preserves catalog data, claims, decisions, media and enrichment history. The discovery job queue remains in memory and is scoped to the single running worker. Waveform analysis runs are persisted per mix record in `analysis_runs`, are read back after a restart, and any run left `queued` or `running` by a previous process is marked `interrupted` on boot rather than being silently lost. The run itself is not resumable. Multi-worker deployment is not supported until the discovery queue is persisted.
