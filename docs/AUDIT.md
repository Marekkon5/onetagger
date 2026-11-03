# Codebase Audit — 2026-11-03

Dated audit of `syco23-mixsets` on branch `syco23-longform-mixsets`, after
merging `origin/master` (`b445059`). Scope: architecture, shared contracts,
server catalog and import plane, worker, client transport/UI, deployment
tooling and tests. This document records **confirmed defects with reproducible
evidence** and separates them from inspected risks and recommended fixes. It
does not change runtime behaviour.

> Historical evidence in `PRODUCTION_VERIFICATION.md` is left untouched. This
> is a separate, dated record.

## Method

- Read the full DOX chain (root `AGENTS.md` plus every child along each
  touched path) before inspection.
- Read the domain contracts, server composition root/routes/catalog/imports/
  providers/jobs, worker control plane and resolvers, client API/store/views,
  deployment Compose/scripts and Playwright specs.
- Re-ran the verification baseline and reproduced each finding with a command,
  a test, or a direct read of the responsible branch.
- Findings are stated against the current working tree, not against the
  historical plan in `docs/superpowers/`.

## Verification baseline

Run from the repository root on 2026-11-03:

| Command | Result |
| --- | --- |
| `pnpm typecheck` | pass |
| `pnpm test` | pass — 222 tests: 12 domain, 60 client, 131 server, 6 worker, 13 deployment |
| `pnpm build` | pass |
| `pnpm test:e2e` | pass — 12/12 on the final run |

The e2e suite contained a structural navigation race (see **T1**). It failed on
two earlier consecutive runs and passed on the final run; the race is now
fixed in the spec and the suite was re-run.

---

## Architecture assessment

The shape is sound and consistent for a single-worker catalog:

- **Contracts first.** `packages/domain` owns strict Zod schemas for records,
  claims, import jobs, evidence and pages; consumers import the built package.
  Unknown keys are rejected, so drift fails loudly at the boundary.
- **One composition root.** `server/src/index.ts` wires one SQLite catalog,
  curator auth and optional import routes; route modules return `boolean` and
  own their CORS/error handling.
- **Clear trust split.** Public reads, curator writes and HMAC-signed worker
  mutations are separate surfaces. Worker reports are re-validated
  server-side (role, object-key shape, SHA-256, size) rather than trusted.
- **Durable queue + stateless worker.** The import outbox row is the queue,
  leases are time-bounded, and the worker never opens SQLite or holds catalog
  state. That is the right direction for a later multi-process deployment.
- **Evidence and merge are explicit.** `FieldDefinitions` is an allow-list;
  merge policy is deterministic and reviewable instead of ad hoc field writes.
- **UI treats the server as canonical.** Mutations update state only after the
  committed record returns, and local browser data is migration-only input.

The weaknesses cluster in the import plane, where the worker, domain and client
were evolved separately and their contracts were not re-tested together. Most
high-severity findings are one or two mismatches at those seams, not a broken
architecture.

### Strengths to preserve

- Strict schemas plus `EvidenceScoredSchema.hardGates` fail closed.
- Object keys derived only from job id, role and content hash; original
  filenames never become storage paths.
- Canonical records never accept a private `urn:` handle; worker storage is
  validated with an allow-list and path traversal checks.
- Curator cookie is HTTP-only, hashed server-side and scoped by origin.
- Migration from the browser library is idempotent and never deletes
  originals.

---

## Findings

Severity reflects user/data impact, not effort.

### High

#### H1 — Finalize ignores real worker metadata and writes placeholder titles

**Files:** `server/src/imports/routes.ts:218-275`,
`server/src/catalog/provider-claims.ts:45-95`,
`worker/src/process.ts:127-147`, `packages/domain/src/fields.ts:12-22`

For catalog providers, finalize derives the title from whether a metadata
artifact exists rather than from its content:

```ts
title: artifacts.find(a => a.role === 'metadata') ? 'Imported mix' : url,
```

`claimsFromImportJob` then reads the evidence field, but
`inferClaimValue('title')` returns `job.sourceUrl`, not the worker's title:

```ts
if (metaArtifact) { ... return job.sourceUrl; }
```

The worker writes only `{ provider, sourceUrl, title, artworkUrl, observedAt }`
to the metadata artifact (`worker/src/process.ts:131-136`) and submits one
evidence item with `field: 'metadata'`. `metadata` is **not** a key in
`FieldDefinitions`, so `claimsFromImportJob` skips it
(`packages/domain/src/provider-claims.ts:56`). Net effect: a metadata import
finalizes as `"Imported mix"` (or the raw URL) with no title/artist/description
claims from the provider.

**Evidence:** `worker/src/process.ts:147`; `packages/domain/src/fields.ts:12-22`
(title exists, `metadata` does not); `server/src/catalog/provider-claims.ts:82-91`.

**Remediation:** have the worker emit per-field evidence (`title`, `description`,
`durationMs`, `genres`, `styles`, `cover`, …) with real values, or store a
structured facts payload and read it during finalize. Make `inferClaimValue`
read the artifact body. Add a finalize test asserting a real title survives.

#### H2 — User-upload finalize cannot satisfy the canonical record contract

**Files:** `server/src/imports/routes.ts:131,219-236`,
`packages/domain/src/schemas.ts` (`ProviderRefSchema`, `CatalogRecordSchema`),
`server/src/catalog/provider-claims.ts:69-70`

For a `user_upload` job, finalize builds a canonical source with:

```ts
provider: 'freeteknomusic',
url: job.sourceUrl,   // urn:syco23:upload:…
```

`CatalogRecordSchema` requires `ProviderRefSchema.url` to be an HTTP URL, so a
`urn:` value fails validation. The record also mislabels a user upload as a
FreeteknoMusic source, and `claimsFromImportJob` turns the URN into
`private://urn:…`, which is not an HTTP `sourceUrl` either.

**Evidence:** reproduced by parsing a canonical record whose only source is
`{ provider: 'freeteknomusic', url: 'urn:syco23:upload:abc', … }` with
`CatalogRecordSchema` → `sources.0.url: Invalid URL`. `ImportCandidateSchema`
also rejects `provider: 'user_upload'`.

**Remediation:** decide the canonical representation for uploads (for example,
a dedicated upload source contract or a `sources` entry with a resolved
playback URL), keep the private URN out of `ProviderRef`, and add an
end-to-end user-upload finalize test that runs `CatalogRecordSchema.parse`.

#### H3 — Client import contracts disagree with the server

**Files:** `client/src/catalog/api.ts:194-200,244`,
`server/src/imports/routes.ts:275`, `server/src/catalog/import-jobs.ts:220`

Two mismatches:

1. `ImportJobDetailSchema.evidence` requires `gates`, but the repository and
   domain emit `hardGates` (`server/src/catalog/repository.ts:81`,
   `packages/domain/src/evidence.ts:39`). Once a job has evidence, polling and
   detail parsing fail validation.
2. `catalogApi.finalize` parses the response as a bare `CatalogRecord`, but the
   server returns `{ record, mixId, claimsApplied }`
   (`server/src/imports/routes.ts:275`). The finalize call therefore throws even
   when the server succeeded.

**Evidence:** direct field comparison; server response object at
`server/src/imports/routes.ts:275`.

**Remediation:** change the client schema to `hardGates`; parse the
`{ record, mixId, claimsApplied }` envelope (matching
`services/api.ts#finalizeImportJob`); add malformed-response tests.

#### H4 — `AUTH_MODE=off` turns every request into a curator

**Files:** `server/src/auth/curator.ts:55-80,109-121`,
`server/src/index.ts:24-29`, `docs/ARCHITECTURE.md:75`

`createCuratorAuth` returns `{ sessionId: 'auth-off' }` for every request when
the mode is `off`. `importsEnabled` only requires `IMPORTS_ENABLED` plus a real
curator hash, so on a deployment with `AUTH_MODE=off` and a configured hash,
`/api/imports` creation/finalize is reachable anonymously. That contradicts
`docs/ARCHITECTURE.md:75` (“`AUTH_MODE=off` deployments cannot expose
acquisition anonymously”).

**Evidence:** `server/src/auth/curator.ts:109-111`; `server/src/index.ts:27`.

**Remediation:** either forbid enabling imports while auth is disabled, or
require an explicit session for import mutations regardless of `AUTH_MODE`.
Correct the architecture claim if the product decision changes.

#### H5 — CORS preflight omits `idempotency-key`

**Files:** `server/src/http.ts:28`, `client/src/catalog/api.ts:279`

The client sends `idempotency-key` on import creation, but the server
advertises only `content-type`:

```ts
response.setHeader('access-control-allow-headers', 'content-type');
```

A browser preflight for a request with a custom `idempotency-key` header will
be rejected, blocking import creation from the deployed client.

**Evidence:** preflight response header observed as
`access-control-allow-headers: content-type`; client header at
`client/src/catalog/api.ts:279`.

**Remediation:** allow `content-type, idempotency-key` (and any future custom
headers) and add a preflight test.

### Medium

#### M1 — Mixcloud is advertised but absent from the import capability matrix

**Files:** `packages/domain/src/import-jobs.ts:66-93`,
`docs/PROVIDERS.md:31`, `README.md`, `server/src/providers/mixcloud.ts`

`ProviderIdSchema` includes `mixcloud` and the docs/UI list it as a
credential-free source, but `ProviderCapabilityMatrixSchema` and
`PROVIDER_CAPABILITIES` omit it. `toCapabilities('mixcloud')` therefore returns
no capability and `decideImportPolicy` blocks metadata import with
`Provider mixcloud does not expose metadata import` (confirmed by calling the
policy directly).

**Remediation:** add an explicit `mixcloud: { metadata: true, audio: false }`
entry, keep `PROVIDERS.md`/README/adapters aligned, and add a policy test.

#### M2 — Worker resolvers do not cover every advertised metadata provider

**Files:** `worker/src/resolvers.ts:29-68`, `packages/domain/src/import-jobs.ts:86-93`

`resolveSource` implements `user_upload`, `soundcloud` and
`archiveorg`/`freeteknomusic`. `youtube`, `hearthis`, `mixcloud` and `discogs`
have metadata capability in the matrix but no worker resolver, so those jobs
cannot complete metadata resolution. Server-side provider adapters exist for
some of these (for example Mixcloud), but the worker does not reuse them.

**Remediation:** add a resolver per provider before claiming capability, with a
worker test per path; or narrow the matrix until resolvers exist.

#### M3 — Worker lease ownership is enforced only on heartbeat

**Files:** `server/src/imports/routes.ts:624-…` (`handleWorker`),
`server/src/imports/routes.ts` `events`/`artifacts`/`evidence`/`complete`

Every worker mutation validates the global HMAC but only `heartbeat` checks
that the caller holds the lease (`x-import-worker-id` + `renewImportLease`).
A valid worker credential can therefore append events/artifacts/evidence or
complete a job owned by another worker.

**Remediation:** require a matching, unexpired lease owner on every mutating
worker action; return 409 otherwise. Add a test for a non-owner attempt.

#### M4 — Request/worker buffers exceed the deployed memory limits

**Files:** `server/src/imports/routes.ts:41`, `server/src/http.ts:43-49`,
`deploy/vps/compose.yml:24,88`, `deploy/vps/.env.example:32-33`

`IMPORT_MAX_INPUT_BYTES` defaults to 2 GiB and `readRequestBytes` buffers the
whole body in memory. The Compose API service is limited to 512 MB, the worker
to 1 GB, and `/tmp` is a 256 MB tmpfs. A large upload can exhaust the container
before the size check helps. Worker fetch/read paths are similarly unbounded
relative to the worker limit.

**Remediation:** enforce chunked/streaming limits, keep the configured maximum
below the container limit, and align `.env.example`, Compose and code defaults.

#### M5 — Curator edits are always attributed to `freeteknomusic`

**Files:** `server/src/catalog/routes.ts:122-135`

`curateRecord` hardcodes:

```ts
provider: { provider: 'freeteknomusic', resourceType: 'curator', externalId: actor }
```

Every curator edit on every record type is recorded as a FreeteknoMusic claim,
which corrupts provenance and evidence displays.

**Remediation:** introduce a `curator` source/provider representation (or use
the record's actual provider) and test the claim provenance.

#### M6 — Duplicate `POST /api/catalog/merge` handler is unreachable

**Files:** `server/src/catalog/routes.ts:186-195` and `415-441`

The first handler matches the path and returns before the second, more complete
handler (which parses revisions and maps errors) is reached. The first throws
`HttpInputError`/`mergeRecords` errors outside a local try/catch, so error
mapping differs from the intended path.

**Remediation:** delete the earlier block or merge the two into one handler and
cover it with the existing merge tests plus an error-path test.

#### M7 — Discovery job map never evicts completed/failed jobs

**Files:** `server/src/jobs/in-memory.ts:30-31,67,82`

`this.jobs`/`this.controllers` grow for the process lifetime; `run()` finalizes
the job but nothing deletes it. Long-running deployments leak memory and keep
stale results addressable.

**Remediation:** evict terminal jobs on a bounded TTL/size, or cap the map and
document the single-worker, in-memory guarantee.

#### M8 — Worker metadata/evidence cannot produce real claims

**Files:** `worker/src/process.ts:127-147`, `packages/domain/src/fields.ts:12-22`,
`server/src/catalog/provider-claims.ts:56`

Related to H1 but distinct: the worker submits a single evidence item with
`field: 'metadata'`, which `FieldDefinitions` does not recognise, so merge
policy ignores it by design. The artifact holds only title/artwork URL, so even
a corrected evidence field has no duration/date/genre/description facts to
score.

**Remediation:** define the metadata fact contract (and field allow-list
entries), have the worker submit per-field evidence with values, and test merge
selection.

#### M9 — `claimsFromImportJob` turns rejected evidence into claims

**Files:** `server/src/catalog/provider-claims.ts:45-77`

The function loops over every scored evidence row and computes
`evidence = auto_apply ? 'direct' : review ? 'parsed' : 'analysis'`, but never
skips rows whose `decision` is `reject`/`blocked`. Rejected worker evidence can
still become a claim, defeating the review gate.

**Remediation:** skip non-apply/non-review decisions before building a claim,
or route them to Review explicitly; assert in a test that rejected evidence
produces no claim.

#### M10 — Client finalize prompt collects a password the server ignores

**Files:** `client/src/views/ImportView.vue:509-511`,
`server/src/imports/routes.ts:180-186`

The UI prompts for `curatorPassword` and posts it, but finalize authorizes via
the curator session (`requireCurator`) and ignores the body password. This is a
misleading control and an unnecessary credential surface in the client.

**Remediation:** remove the prompt (session already authorizes) or make the
server verify it; update the client test.

#### M11 — “Already finalized” check reads the wrong object

**Files:** `client/src/views/ImportView.vue:511`

```ts
importJobs.value.find(entry => entry.job.id === job.job.id && (entry as any).mixId)
```

The `mixId` lives on `entry.job.mixId` (or the store job), not on `entry`.
The guard never fires, so finalize is offered again for already-linked jobs.

**Remediation:** check `entry.job.mixId` and a typed view-model field.

#### M12 — S3-compatible store does not sign requests

**Files:** `worker/src/storage.ts:70-84`

`S3ObjectStore` accepts `accessKey`/`secretKey` but issues unsigned `PUT`/`GET`
requests. Against a real private bucket it fails; against a lenient endpoint it
writes without authentication. The code path is not production-ready.

**Remediation:** implement AWS Signature V4 (or use an SDK) and test against a
local S3-compatible service; otherwise keep the local store as the only
supported backend and say so.

### Low

#### L1 — `readRequestBytes` has no per-route body limit documentation

**File:** `server/src/http.ts:33-49`

`readJsonBody` defaults to 2 MB, which is sensible, but import routes use the
2 GiB limit and there is no shared table of route limits. Document limits next
to the route contract to prevent future drift.

#### L2 — Discovery/worker state changes are not observable after restart

**Files:** `server/src/jobs/in-memory.ts`, `server/src/imports/routes.ts`

Discovery jobs are in-memory by design; the import queue is durable. The
distinction is documented, but a restart silently loses discovery progress.
Surface that in the UI/health output rather than implying durability.

#### L3 — Provider docs and capability matrix are maintained in three places

**Files:** `README.md`, `docs/PROVIDERS.md`,
`packages/domain/src/import-jobs.ts`, `server/src/providers/*`

The same provider list is asserted in docs, the domain matrix and adapters.
M1 is a direct result. Prefer one capability source (the domain constant) and
generate/validate the docs list from it.

### Test findings

#### T1 — e2e navigation race in the discovery import spec

**File:** `tests/e2e/catalog.spec.ts:94-107`

After clicking “Open indexed”, the test asserts a heading with the same text
that is also present on `/import`, then captures `page.url()`. If the heading
matches before the route change commits, `detailUrl` is `/import` and the public
context later opens the wrong page.

Observed: two consecutive failing runs earlier in the session; a later run
passed 12/12. This is timing-dependent, not deterministic.

**Status: fixed (test-only).** The spec now waits for the detail URL before
capturing it:

```ts
await page.getByRole('link', { name: 'Open indexed', exact: true }).click();
await expect(page).toHaveURL(/\/mix\//);
await expect(page.getByRole('heading', { name: 'Imported Click Recording', exact: true })).toBeVisible();
const detailUrl = page.url();
```

Re-run `pnpm test:e2e` after the fix; the suite passed 12/12.

#### T2 — No end-to-end user-upload or finalize-with-evidence test

The failing paths in H1/H2/H3 are exactly the ones without route-level tests.
Add: user-upload create → finalize → `CatalogRecordSchema.parse`; metadata job
with evidence → detail poll → finalize → real title/claims.

#### T3 — Import route tests do not assert CORS or lease ownership

Add a preflight assertion for `idempotency-key` (H5) and a non-owner worker
mutation test (M3).

---

## Remediation priorities

1. **Stop data corruption / broken flows first**
   - H2 (user-upload finalize), H3 (client/server contract), H1 (real metadata).
   - Add T2 tests with each fix.
2. **Close access and transport gaps**
   - H4 (`AUTH_MODE=off`), H5 (CORS), M3 (lease ownership).
3. **Make capabilities honest**
   - M1 (Mixcloud matrix), M2 (worker resolvers), M8 (metadata facts).
4. **Bound resources and provenance**
   - M4 (memory), M5 (curator attribution), M6 (merge handler), M9 (rejected
     evidence), M12 (S3 signing).
5. **Hygiene**
   - M7 (job eviction), M10/M11 (client finalize UX), L1-L3, T1.

Suggested gates before the next release: `pnpm typecheck && pnpm test &&
pnpm build && pnpm test:e2e`, plus new regression tests for H1-H5.

---

## Reproducible evidence index

| Finding | Reproduce |
| --- | --- |
| H1 | Read `server/src/imports/routes.ts:245` and `server/src/catalog/provider-claims.ts:82-91`; `metadata` absent from `packages/domain/src/fields.ts:12-22`. |
| H2 | `CatalogRecordSchema.parse` a record whose source URL is `urn:syco23:upload:abc` → `sources.0.url: Invalid URL`. |
| H3 | Compare `client/src/catalog/api.ts:244` (`gates`) with `packages/domain/src/evidence.ts:39` (`hardGates`); compare `api.ts:199` with `routes.ts:275`. |
| H4 | `server/src/auth/curator.ts:109-111` returns `auth-off`; `server/src/index.ts:27` gates imports on the hash only. |
| H5 | Preflight response `access-control-allow-headers: content-type`; client sends `idempotency-key` at `client/src/catalog/api.ts:279`. |
| M1 | Call `decideImportPolicy` with `provider: 'mixcloud', mode: 'metadata'` → `allowed: false, reason: "Provider mixcloud does not expose metadata import"`. |
| M2 | `worker/src/resolvers.ts` has no branch for `youtube`/`hearthis`/`mixcloud`/`discogs`. |
| M3 | `handleWorker`: only the `heartbeat` branch calls `renewImportLease`. |
| M4 | `IMPORT_MAX_INPUT_BYTES` default 2 GiB vs `compose.yml` 512 MB/1 GB. |
| M5 | `curateRecord` hardcodes `provider: 'freeteknomusic'`. |
| M6 | Two `POST /api/catalog/merge` blocks at `routes.ts:186` and `415`. |
| M7 | `server/src/jobs/in-memory.ts` sets but never deletes map entries. |
| T1 | `tests/e2e/catalog.spec.ts:94-107` captures `page.url()` without a URL assertion. |

## Limitations

- No load/soak test was run; memory findings are from limits and buffering
  code, not measured exhaustion.
- No real S3/MinIO integration test was run; M12 is a code-level finding.
- Provider live behaviour was not exercised against the network; findings are
  contract-level.
- The merge commit `b445059` is local and was not pushed as part of this audit.

---

# Remediation record — 2026-11-03

Every finding above has a code fix and, where behaviour is route- or
contract-level, a regression test. This section is additive; the findings and
the original verification baseline are preserved unchanged.

## Verification after remediation

Run from the repository root:

| Command | Result |
| --- | --- |
| `pnpm typecheck` | pass |
| `pnpm test` | pass — 242 tests: 15 domain, 64 client, 138 server, 12 worker, 13 deployment |
| `pnpm build` | pass |
| `pnpm test:e2e` | pass — 12/12 |
| `IMPORT_WORKER_SECRET=… docker compose -f deploy/vps/compose.yml config` | pass |

The Compose validation requires `IMPORT_WORKER_SECRET` because the worker fails
closed without it; the command was run with a throwaway value.

## Finding-by-finding

### High

- **H1 — real metadata at finalize.** Metadata and audio jobs now snapshot
  `ImportMetadataFactsSchema` (title, artists, crews, uploader, description,
  duration, dates, genres, styles, artwork, external id) and submit per-field
  evidence carrying the sourced `value`. `server/src/imports/finalize.ts` reads
  the `metadata` artifact with a 256 KiB cap and hash verification, and
  `handleFinalize` builds the record and claims from those facts. A provider job
  without a readable snapshot returns `409`; there is no placeholder title.
  Test: *metadata finalize uses the worker facts for the real title and claims*;
  *provider finalize refuses jobs without a readable metadata snapshot*.
- **H2 — valid user-upload finalize.** A user upload becomes a canonical mix
  with `sources: []`; the private `urn:syco23:upload:…` handle stays only in the
  import job and provenance tables. Finalize requires a title from the body or
  the worker snapshot and accepts an optional HTTP `playbackUrl`. Test:
  *user-upload finalize creates a valid record without a private URN source*.
- **H3 — client/server contracts.** `ImportJobDetailSchema.evidence` uses
  `hardGates`, and `catalogApi.finalize` parses
  `ImportFinalizeResultSchema` (`{ record, mixId, claimsApplied }`). Client API
  tests cover the envelope and evidence shape.
- **H4 — `AUTH_MODE=off`.** `importsEnabled` now uses `resolveImportGate`,
  requiring `IMPORTS_ENABLED`, a real `scrypt$…` hash and
  `curatorAuth.mode === 'on'`. `AUTH_MODE=off` cannot expose acquisition;
  startup warns when imports were requested but the gate is closed. Tests:
  *imports stay dark under AUTH_MODE=off even with a configured curator hash*;
  *imports require an explicit request flag, a real hash and AUTH_MODE=on*.
- **H5 — CORS preflight.** Both the shared CORS helper and the composition-root
  JSON response advertise `content-type, idempotency-key`. Test: *CORS preflight
  allows the idempotency-key header*.

### Medium

- **M1 — Mixcloud capability.** `PROVIDER_CAPABILITIES` includes
  `mixcloud: { metadata: true, audio: false }`. Policy and docs agree.
- **M2 — worker resolver coverage.** Resolvers exist for `soundcloud`,
  `youtube`, `hearthis`, `mixcloud`, `archiveorg`, `freeteknomusic` and
  `user_upload`, each emitting facts. Test: *every metadata-capable provider has
  a worker resolver*.
- **M3 — lease ownership.** Events, artifacts, evidence and completion all
  require the `x-import-worker-id` that holds an unexpired lease and return
  `409` otherwise, not just heartbeat. Test: *worker mutations require the
  current lease owner*.
- **M4 — bounded memory.** Import defaults are 256 MiB input and 256 MiB output;
  bodies are rejected while streaming past the cap. Compose uses API 768 MB,
  worker 1.5 GB and a 640 MB `/tmp`; `.env.example`, Compose and code defaults
  agree.
- **M5 — curator provenance.** `ClaimProviderIdSchema` adds `curator`;
  curator edits are recorded as `{ provider: 'curator', resourceType:
  'curation' }`, and enrichment reports filter curator claims rather than
  mislabelling them as a provider.
- **M6 — one merge handler.** The unreachable earlier `POST /api/catalog/merge`
  block is deleted; the remaining handler requires positive revisions and maps
  errors locally.
- **M7 — discovery eviction.** Terminal discovery jobs are removed after a
  30-minute TTL and the retained terminal set is capped at 200 on every
  list/get/create.
- **M8 — metadata facts and claims.** `ImportMetadataFactsSchema` is the shared
  fact contract; migration `011-evidence-values.sql` persists evidence values;
  `FieldDefinitions` remains the claim allow-list and merge selects per field.
- **M9 — rejected evidence.** `claimsFromImportJob` skips `decision: 'reject'`
  before building a claim. Test: *rejected worker evidence never becomes a
  claim*.
- **M10 — no password prompt.** Finalize authorizes via the curator session only;
  the client sends an optional `{ title, playbackUrl }` body and no credential.
- **M11 — already-finalized guard.** The client checks `entry.job.mixId`, so a
  linked job is not offered for finalize again.
- **M12 — SigV4 signing.** Both `server/src/imports/storage.ts` and
  `worker/src/storage.ts` sign `PUT`/`GET` with AWS Signature V4 and require
  credentials in S3 mode; `IMPORT_S3_REGION` is configured. Test: *S3 requests
  carry a deterministic AWS Signature V4*.

### Low and tests

- **L1 — route limits documented.** `importLimits()` states the memory rationale
  and the `.env.example`/Compose limits match it.
- **L2 — non-durable discovery surfaced.** `/api/health` returns
  `discovery: { durable: false, running, pending, retained }`; the import view
  states that discovery is not durable.
- **L3 — one capability source.** `PROVIDER_CAPABILITIES` is documented as the
  source of truth, with a matching capability table in `PROVIDERS.md` and
  README. Resolver coverage is tested against the matrix.
- **T1 — e2e race.** The spec waits for `/mix/` before capturing the detail URL;
  the suite passes 12/12.
- **T2 — end-to-end finalize coverage.** Server route tests cover user-upload
  finalize and metadata-with-evidence finalize; the client parses both.
- **T3 — CORS and lease tests.** The preflight and non-owner worker-mutation
  tests are in `server/src/imports/routes.test.ts`.

## Residual notes

- The memory ceiling is enforced by a streaming byte counter plus conservative
  quotas, not by streaming uploads to disk; a future large-upload path should
  stream to storage.
- SigV4 is unit-tested for deterministic signing, not against a live MinIO/S3
  endpoint. Keep the local store as the default until that integration is run.
- Discovery remains process-local by design; the queue is durable, discovery is
  not.
