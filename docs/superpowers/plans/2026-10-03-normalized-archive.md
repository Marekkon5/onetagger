# Normalized Shared Archive Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver one persistent, curated Mixsets archive with normalized mix, artist, crew, label and event indexes, source evidence and protected enrichment.

**Architecture:** Shared runtime schemas validate catalog records and provider claims. A SQLite repository on VPS-L owns identities, relationships, evidence and atomic merges; Vue reads and edits that catalog through authenticated curator routes. Existing providers, Vercel hosting and bounded waveform processing remain the foundation.

**Tech Stack:** Vue 3/Quasar, TypeScript, Zod 4, Node 22 with built-in SQLite, pnpm workspaces, existing Node test runner/tsx, Vitest for client state tests, Docker/Caddy and Vercel. Pin the production Node image to the currently verified 22.23.3 release and lock compatible dependency versions.

**Spec:** `docs/superpowers/specs/2026-10-03-normalized-archive-design.md`

## Implementation status — 2026-10-05

Reconciled against the codebase on `master` (`2a26b10`). The plan was executed on `master` and integrated through `syco23-longform-mixsets` / PR #1 and later hardening commits, **not** through the per-task commit messages written in the tasks below.

- **Tasks 1–8: complete** (all boxes below ticked). Key commits: `bba3cb3` (normalized catalog + local MP3 tagging), `1da541c`, `c91a43e`, `24bd332`, `3c26305`, `898ece5` (normalized catalog, review workflow and e2e coverage), `39e4551`.
- **Task 9: complete (2026-10-05).** `master` was promoted to production and verified (backend + Vercel frontend, deployed-browser click-through), live provider enrichment was verified, and a production import with independent-session verification and a controlled conflicting-claim exercise was performed; see `docs/PRODUCTION_VERIFICATION.md`.
- **File-name drift (plan → implementation):**
  - Task 3 `server/src/catalog/review.ts` → folded into `merge.ts` / `repository.ts` (no separate `review.ts`).
  - Task 7 `client/src/catalog/migration.ts` → folded into `store.ts` (no separate `migration.ts`).
  - Task 8 per-type views (`EntityIndexView`, `EntityDetailView`, `EventIndexView`, `EventDetailView`) → generic `client/src/views/CatalogIndexView.vue` + `CatalogDetailView.vue`; the planned short routes (`/artists`, `/crews`, `/labels`, `/events`, `/entity/:id`, `/event/:id`) are unchanged.
  - Task 1 `packages/domain/src/normalize.test.ts` → folded into `schemas.test.ts`.
  - Extra tests beyond the plan: `server/src/catalog/api-gaps.test.ts`, `server/src/catalog/completeness.test.ts`, `server/src/http.test.ts`, provider `artwork.test.ts` / `source-refresh.test.ts`, `server/src/jobs/in-memory.test.ts`, `client/src/local-mp3.test.ts`.
- **Related:** a parallel local-only implementation of this same plan (`codex/normalized-archive`, Tasks 1–7 only) was archived as tag `archive/codex-normalized-archive-1a4d70a` and not merged.

## Global Constraints

- "Provider credentials remain on the worker and must never enter frontend configuration, raw claim payloads or logs."
- "Network requests and audio decoding happen outside database transactions."
- "Matching names alone do not establish an additional role."
- "Names such as `DJ ...` or `Live ...` must not lose meaningful words through the current search-query normalizer."
- "Confidence can prioritize review; confidence alone is not proof."
- "Recording date, upload date and event date are distinct fields."
- "A name inferred from a filename cannot establish a confirmed artist or crew identity by itself."
- "Existing good covers stay selected; a different proposed cover goes to Review."
- "Other users' browser-local records cannot be uploaded from this agent's session. Each browser must perform its own authorized migration; the frontend provides that routine."
- "SoundCloud-wide official search remains unavailable until its client secret or a supported access token is configured."

## Review Focus

- Two provider/resource namespaces issue the same external ID: preserve both identities; Task 2 tests this.
- A filename suggests an artist, crew or event with a plausible exact-name match: keep unconfirmed identity/role proposals out of confirmed indexes; Tasks 3 and 5 test this.
- A provider returns an upload date, uploader avatar or channel name: preserve their actual meaning and never promote them to recording date, cover or performer; Tasks 1 and 5 test this.
- Two browsers retry an import/migration or edit during enrichment: converge on one source identity and reject stale changes without dropping curation; Tasks 3, 4 and 6 test this.
- Worker recreation or a failed browser migration interrupts a write: preserve durable data and local originals, report interruption honestly and allow an idempotent retry; Tasks 6, 7 and 9 test this.

## File boundaries and common interfaces

- `packages/domain/src/{schemas,normalize,fields,index}.ts`: shared canonical contracts, validation and field definitions. Public exports come from `index.ts`.
- `server/src/catalog/{database,repository,identity,merge,review,routes,enrich,migration,media,backup}.ts`: one responsibility per file; no provider HTTP calls inside repository/merge transactions.
- `server/src/auth/{curator,routes}.ts`: password/session handling and access routes.
- `client/src/catalog/{api,store,views,migration}.ts`: transport, authoritative state, compatibility display projection and local migration conversion. The existing `useMixStore` becomes a facade rather than a second merge engine.
- `client/src/views/{EntityIndex,EntityDetail,EventIndex,EventDetail,CuratorLogin}View.vue`: catalog views. Existing mix/import/review screens consume the same catalog state.
- `deploy/vps`: persistent volume, privately configured curator, consistent backup/restore and deployment instructions.

> **Drift (2026-10-05):** the implementation consolidated some planned files — `server/src/catalog/review.ts` → `merge.ts` / `repository.ts`; `client/src/catalog/migration.ts` → `store.ts`; the per-type `EntityIndexView` / `EntityDetailView` / `EventIndexView` / `EventDetailView` → the generic `CatalogIndexView.vue` / `CatalogDetailView.vue`. See the "Implementation status — 2026-10-05" section above.

Task 1 defines these shared types: `RecordId`, `EntityRole` (`artist | crew | label`), `IndexKind` (`mix | artist | crew | label | event`), `CatalogRecord` (`MixRecord | EntityRecord | EventRecord`), `CatalogDetail`, `ProviderRef`, `FieldClaim`, `ReviewItem`, `EnrichmentReport`, `ImportResult`, `MigrationResult`, `PageQuery` and `CatalogPage`. `RecordId` is a stable opaque string; new records use UUIDs, and legacy IDs are aliases. Entity roles share a canonical entity ID. Record revisions are positive integers.

Use `verification: proposed | source-confirmed | curator-confirmed` separately from `reviewState: ready | review`. Claim evidence is `direct | parsed | curated | analysis`; claim disposition is `selected | corroborated | pending | rejected`. Field definitions determine allowed target types and normalized value schemas. Only direct facts about a confirmed identity, confirmed recording-source facts or curator decisions can select missing fields automatically.

Public API routes are `GET /api/catalog/:index`, `GET /api/catalog/records/:id` and `GET /api/catalog/media/:id` for published media. `GET /api/catalog/review` requires a curator. Mutation routes are `POST /api/catalog/import`, `POST /api/catalog/migrate`, `POST /api/catalog/records/:id/enrich`, `PATCH /api/catalog/records/:id`, `POST /api/catalog/review/:id/decision` and `POST /api/catalog/merge`. Authentication routes are `POST /api/auth/login`, `POST /api/auth/logout` and `GET /api/auth/session`. JSON reads return schema-validated canonical details; writes return the committed result or an explicit validation/access/revision error. API-controlled revisions, evidence and record verification cannot be changed through an unrestricted client patch.

## Task 1: Shared catalog contracts and normalization

**Files:** Create `packages/domain/package.json`, `packages/domain/tsconfig.json`, `packages/domain/src/schemas.ts`, `normalize.ts`, `fields.ts`, `index.ts`, `schemas.test.ts` and `normalize.test.ts` *(implemented: folded into `schemas.test.ts`)*. Modify `pnpm-workspace.yaml`, `package.json`, `client/package.json`, `server/package.json`, `pnpm-lock.yaml`, `client/src/domain/types.ts` and `server/src/domain.ts`.

**Interfaces:** Produce `CatalogRecordSchema`, `FieldClaimSchema`, the common types above, `normalizeName(value: string): string`, `normalizeProviderRef(ref: ProviderRef): ProviderRef`, `validateField(record: CatalogRecord, field: string, value: unknown): unknown` and `missingFields(record: CatalogRecord): string[]`. Retain an explicit `LegacyMixSchema` for migration; keep provider discovery DTOs separate from canonical records.

- [x] Write schema/normalization tests asserting that `DJ Live` remains meaningful, composed/decomposed Unicode names compare consistently, display spelling survives, unknown canonical keys and invalid dates/countries/durations fail validation, and upload/recording dates have separate fields. Assert that portrait and cover are distinct asset roles and missing-field calculations do not count a portrait as a cover.
- [x] Run `pnpm --dir packages/domain test`; confirm the tests fail before the exports exist.
- [x] Implement the shared contracts and per-field value schemas, including date precision, type-specific entity details, typed links, normalized sets and bounded URLs/arrays. Add workspace build/typecheck/test scripts; use a compiled package export so Node runtime imports do not depend on TypeScript source.
- [x] Run the domain tests and domain build/typecheck; confirm all assertions pass and both consumers resolve the workspace package.
- [x] Commit as `feat: add shared normalized catalog contracts`.

## Task 2: Durable relational repository and identity constraints

**Files:** Create `server/src/catalog/database.ts`, `repository.ts`, `repository.test.ts` and `migrations/001-catalog.sql`. Modify `server/src/index.ts` only to initialize/close the repository, and `server/Dockerfile` to package workspace runtime dependencies.

**Interfaces:** Produce `openCatalog(path: string): CatalogRepository`; methods `getRecord(id: RecordId): CatalogDetail | undefined`, `listIndex(kind: IndexKind, query: PageQuery): CatalogPage`, `findByProvider(ref: ProviderRef): RecordId | undefined`, `findBySource(ref: ProviderRef): RecordId | undefined`, `findNameCandidates(role: EntityRole, name: string): EntityRecord[]`, `transaction<T>(operation: (tx: CatalogTransaction) => T): T` and `close(): void`. Transaction methods save typed records, sources, claims, relationships and legacy aliases with revision checks.

- [x] Write on-disk repository tests that close/reopen the database and retain links/evidence; assert the same `(provider, resourceType, externalId)` cannot own two records, while equal IDs across different namespaces can. Assert wrong target roles/dangling relationships fail and proposed entities are excluded from default public indexes.
- [x] Run `pnpm --dir server exec tsx --test src/catalog/repository.test.ts`; confirm failures before repository implementation.
- [x] Implement strict relational tables from the spec, schema migrations, prepared statements, foreign keys, WAL and a bounded busy timeout. Add indexed name/alias lookup without global name uniqueness. Paginate lists with a maximum page size of 50. Keep transactions synchronous and short; roll back all changes on errors.
- [x] Run repository tests and server typecheck; reopen a test database and confirm returned records pass shared schemas.
- [x] Commit as `feat: persist normalized catalog identities and evidence`.

## Task 3: Identity resolution, canonical merge and durable Review

**Files:** Create `server/src/catalog/identity.ts`, `merge.ts`, `review.ts`, `merge.test.ts`, `identity.test.ts` and `review.test.ts`. *(implemented: no separate `review.ts`; review logic lives in `merge.ts` / `repository.ts`.)*

**Interfaces:** Consume Task 2's repository and Task 1's contracts. Produce `importCandidate(repo: CatalogRepository, candidate: MixCandidate, actor: CuratorActor): ImportResult`, `applyClaims(repo: CatalogRepository, claims: FieldClaim[]): EnrichmentReport`, `decideReview(repo: CatalogRepository, id: string, decision: 'accept' | 'reject', expectedRevision: number, actor: CuratorActor): CatalogDetail` and `mergeRecords(repo: CatalogRepository, survivor: RecordId, duplicate: RecordId, expectedRevisions: [number, number], actor: CuratorActor): CatalogDetail`. `CuratorActor` identifies the authenticated curator session without carrying secrets.

- [x] Write tests asserting repeated source imports return one ID; fuzzy equal-name artist/event proposals remain pending; known source identities resolve consistently; existing cover/profile/country values survive disagreements; equivalent normalized claims corroborate; repeated/rejected claims do not reappear unchanged; stale review acceptance fails; duplicate merge retains evidence and legacy redirects.
- [x] Run the three catalog test files; confirm failures before implementing the functions.
- [x] Implement identity resolution and one field-level merge policy. Use stable claim fingerprints and transactionally selected evidence. Source links and confirmed aliases are additive; differing performer lists, genres/styles, dates and selected media require review. Parsed facts stay proposals until a curator confirms them. Explicit curator imports record which visible facts were confirmed rather than marking every inferred fact as verified.
- [x] Run catalog tests and server typecheck; confirm selected fields and review decisions survive repository reopen.
- [x] Commit as `feat: merge sourced claims with protected canonical values`.

## Task 4: Curator sessions and validated catalog API

**Files:** Create `server/src/auth/curator.ts`, `routes.ts`, `curator.test.ts`, `server/src/catalog/routes.ts` and `routes.test.ts`. Modify `server/src/index.ts`; add `deploy/vps/setup_curator.py`, reusing the existing atomic private configuration helper.

**Interfaces:** Produce `createCuratorAuth(repo: CatalogRepository, passwordHash: string): CuratorAuth` with `login(password: string): SessionCookie`, `authenticate(request: IncomingMessage): CuratorActor | undefined` and `logout(request: IncomingMessage): void`. Produce `handleCatalogRoute(context: ApiContext, req: IncomingMessage, res: ServerResponse): Promise<boolean>` and a shared response/input-validation helper. `ApiContext` contains repository, authentication and registry; never raw credentials.

- [x] Write HTTP/auth tests asserting public reads work, mutations/search/enrichment/analysis require a curator, wrong passwords fail with rate limits, session tokens are hashed at rest, unapproved origins fail writes, cookie authentication/CORS works for approved origins, malformed/oversized inputs return validation errors, and stale edits return HTTP 409.
- [x] Run the auth/routes tests; confirm failures before implementation.
- [x] Implement scrypt password hashing, a bounded session lifetime, secure HTTP-only cookies, logout and exact-origin checks. Bound JSON requests at 2 MiB and validate with shared schemas. Allow credentialed CORS only for configured exact origins; include PATCH in preflight methods. Protect existing expensive discovery/enrichment/waveform endpoints as well as catalog writes. The private setup routine validates password confirmation and stores only the hash in the worker configuration.
- [x] Run tests and typechecks; start the local API with a disposable curator password and verify read/login/write/logout without displaying credentials in output.
- [x] Commit as `feat: add curator access and validated catalog API`.

## Task 5: Provider-wide claims for normalized entities and mixes

**Files:** Create `server/src/catalog/enrich.ts`, `provider-claims.ts` and `enrich.test.ts`. Modify `server/src/core/enrichment.ts`, `providers/discogs.ts`, `providers/soundcloud.ts`, `providers/youtube.ts`, `providers/hearthis.ts`, `providers/archiveorg.ts`, `providers/freeteknomusic.ts` and their relevant provider tests.

**Interfaces:** Produce `enrichCatalogRecord(repo: CatalogRepository, registry: ProviderRegistry, id: RecordId, actor: CuratorActor): Promise<EnrichmentReport>` and `claimsFromProvider(target: CatalogDetail, result: ProviderMetadata): FieldClaim[]`. Define `ProviderMetadata` as a validated adapter result with resource identity, structured facts, sourced relationships and parsed proposals. Extend Discogs with ID-based artist/label hydration, preserving its current search interface for unresolved proposals.

- [x] Write provider/merge integration tests asserting confirmed Discogs hydration updates one entity referenced by two mixes; explicit aliases/groups/members and label hierarchy become typed claims; a label mention does not prove a mix-label relationship; same-name ambiguity goes to Review; existing source providers can still refresh missing metadata; one failed provider does not discard successful claims. Assert SoundCloud `metadata_artist` differs from uploader and that upload dates/channel names/avatar/generic logos cannot become recording dates/performers/covers.
- [x] Run enrichment and relevant provider tests; confirm the new integration cases fail before implementation.
- [x] Implement capability-scoped enrichment for mixes and entity/event records, using identity evidence and Task 3's merge policy. Search only usable provider capabilities; hydrate known IDs directly. Normalize Discogs profile markup while retaining its source. Free-text event/venue/country/crew extraction yields review proposals with excerpts. Persist run results and interruption states. Known recording artwork can fill a missing cover; different selected artwork goes to Review.
- [x] Run meaningful provider/enrichment tests and typechecks; execute a controlled live Kan10 request and confirm YouTube artwork/Discogs data become valid claims without mutating unrelated fields. SoundCloud remains explicitly limited without its secret.
- [x] Commit as `feat: enrich normalized catalog records across providers`.

## Task 6: Safe browser migration, persistent media and backup

**Files:** Create `server/src/catalog/migration.ts`, `media.ts`, `backup.ts`, `migration.test.ts`, `media.test.ts` and `backup.test.ts`. Modify `server/src/core/waveform.ts`, `catalog/routes.ts`, `deploy/vps/compose.yml`, `.env.example` and `server/Dockerfile`. Create `deploy/vps/backup_catalog.py` and `restore_catalog.py`.

**Interfaces:** Produce `migrateLegacyLibrary(repo: CatalogRepository, records: unknown[], batchId: string, actor: CuratorActor): MigrationResult`, `persistWaveform(repo: CatalogRepository, mixId: RecordId, png: Uint8Array, sourceUrl: string): CatalogDetail`, `backupCatalog(repo: CatalogRepository, destination: string): Promise<void>` and `restoreCatalog(source: string, destination: string): Promise<void>` for a stopped target. Migration returns per-record outcomes and legacy-ID mappings.

- [x] Write tests that migrate the same batch/source from two browsers once, preserve manual covers/waveform/review decisions, omit known demo fixtures, retain unverified field evidence, reject invalid/colliding legacy records without losing valid records, reject oversized/non-PNG waveform data, and restore a consistent backup with links/decisions intact.
- [x] Run migration/media/backup tests; confirm failures before implementation.
- [x] Implement per-record migration outcomes with idempotent batch IDs and legacy aliases. Validate generated media before storing files beneath a persistent data directory; prevent path traversal and use atomic writes. Mount `/app/data` on a Docker volume owned by the worker user; store database/media there. Preserve current 250 MiB/five-minute audio limits. Use SQLite's online backup facility and verify backups before restore. Keep failed/interrupted analyses visible and retryable.
- [x] Run tests and a local container recreation/restore exercise; confirm catalog IDs and waveform files survive. Verify secrets and the persistent volume are absent from the image build context.
- [x] Commit as `feat: migrate local archives and persist catalog media`.

## Task 7: Frontend curator login, shared store and migration

**Files:** Create `client/src/catalog/api.ts`, `store.ts`, `views.ts`, `migration.ts`, `store.test.ts`, `migration.test.ts` *(implemented: no separate `migration.ts`; migration lives in `store.ts`.)* and `client/src/views/CuratorLoginView.vue`. Modify `client/src/services/api.ts`, `composables/useMixStore.ts`, `domain/types.ts`, `App.vue`, `scripts/router.ts`, `views/ImportView.vue`, `views/MixDetailView.vue`, `client/package.json` and `pnpm-lock.yaml`.

**Interfaces:** Produce `useCatalogStore()` with `loadIndex(kind: IndexKind, query: PageQuery)`, `loadDetail(id: RecordId)`, `importCandidate(candidate: MixCandidate)`, `enrichRecord(id: RecordId)`, `updateRecord(id: RecordId, patch: unknown, revision: number)`, `login(password: string)`, `logout()` and `migrateLocalLibrary()`. Produce `toMixView(detail: CatalogDetail): MixSetView`; name arrays/profiles in this view are derived, never another persisted canonical entry. Transport uses `credentials: 'include'` and shared response schemas.

- [x] Write client state tests with mocked HTTP responses asserting server records control state; unauthenticated writes lead to login; failed/stale writes do not appear saved; missing-field/results wording distinguishes nothing added from no missing fields; migration retains local originals on partial failure and retries with the same batch ID; malformed server responses fail visibly.
- [x] Run `pnpm --dir client test`; confirm new state tests fail before implementation.
- [x] Implement the authenticated catalog transport/store and convert existing actions to use returned committed records. Remove the browser's canonical merge/dedup engine; keep local cache solely for reads/migration. Add login/logout and an explicit real-record migration summary. Resolve legacy detail IDs through the API. Optional auto-enrichment after import uses the server flow and shows remaining missing fields/provider failures/review links.
- [x] Run client tests/typecheck/build; verify login → migrate/import → enrich → reload through the available browser, using private disposable local credentials.
- [x] Commit as `feat: connect Mixsets frontend to the shared curator archive`.

## Task 8: Entity/event indexes, curator editing and unified Review

**Files:** Create `client/src/views/EntityIndexView.vue`, `EntityDetailView.vue`, `EventIndexView.vue`, `EventDetailView.vue` *(implemented as the generic `CatalogIndexView.vue` / `CatalogDetailView.vue`)*, `client/src/components/FieldEvidence.vue`, `CatalogEditor.vue` and `client/src/catalog/review.test.ts`. Modify `App.vue`, `scripts/router.ts`, `views/LibraryView.vue`, `views/MixDetailView.vue`, `views/ReviewView.vue`, `style/app.scss` and `catalog/api.ts`, `store.ts`.

**Interfaces:** Extend store with `loadReview()`, `decideReview(id: string, decision: 'accept' | 'reject', revision: number)` and `mergeRecords(survivor: RecordId, duplicate: RecordId, revisions: [number, number])`. Routes are `/artists`, `/crews`, `/labels`, `/events`, `/entity/:id`, `/event/:id` and `/login`, alongside existing mix routes. API filters use Task 1's `IndexKind`.

- [x] Write review/view-projection tests asserting the right type-specific fields/evidence appear, entity links shared by two mixes resolve to one ID, rejected/stale decisions remain truthful, and event dates never replace mix recording dates. Assert index totals exclude unresolved proposals for public readers.
- [x] Run client tests; confirm new review/projection cases fail before implementation.
- [x] Implement searchable paginated entity/event indexes and details, typed curator editing, relationship navigation and field-level Review with current/proposed values, source links and match explanations. Add duplicate-merge preview with both revisions. Keep all controls usable on desktop and narrow layouts, including source images, table alternatives and navigation.
- [x] Run client tests/build; click through every new index/detail/editor and a real conflict acceptance/rejection. Verify an independent browser can open committed mix/entity/event links and that public readers see confirmed data without edit actions.
- [x] Commit as `feat: add normalized entity indexes and unified metadata review`.

## Task 9: Production rollout and complete verification

**Files:** Modify `README.md`, `docs/ARCHITECTURE.md`, `docs/PROVIDERS.md`; create `docs/PRODUCTION_VERIFICATION.md`. Deployment uses the existing VPS-L and Vercel setup.

**Interfaces:** Consume all preceding tasks. Produce a working production archive and a dated verification record with concrete IDs, URLs, provider outcomes, schema/merge checks, migration results, persistence/restore results and any remaining credential/browser limitation.

- [x] Run workspace tests/typechecks/builds, existing provider/analysis tests and the local backup/recreation exercise. Confirm the repository contains no credentials and the frontend bundle contains no private provider/auth data. Resolve only concrete failures or required gates before rollout.
- [x] Back up the existing private configuration and any catalog data, deploy the worker/schema/media volume, configure the curator privately, and verify domain TLS, exact-origin credentialed CORS, public reads and authenticated writes before changing the frontend.
- [x] Deploy the frontend to Vercel production with the custom API domain. *(Done 2026-10-05: the `syco23-mixsets` Vercel project auto-deploys the `master` branch to `https://mixsets.syco23.org`; the 4 canonical records were already present and were not re-migrated in this run.)* Migrate this session's real records through the UI; retain the originals until the returned migration summary confirms success. The user's separate browser remains able to perform its own migration.
- [x] Search Mackitek, import the first qualifying real result, run enrichment, inspect claims and remaining gaps, and verify its shared detail from an independent public session. Separately verify the known Kan10 recording automatically receives its matching YouTube cover and Discogs entity profile. Trigger controlled conflicting claims to prove selected data protection/Review, then remove only disposable test records through authenticated routines. *(Done 2026-10-05: live Mackitek search; a production import ("01h20 dans le rouge") read back from an independent, unauthenticated session; a controlled duration conflict created by merging a real Mixcloud duplicate, with the survivor's selected value protected and the item then rejected; cleanup left only the real import. A merge defect found here was fixed in `347cfc4`. See `docs/PRODUCTION_VERIFICATION.md`.)*
- [x] Verify entity/event navigation and curator flows on desktop and a controllable mobile viewport. *(Done 2026-10-05: headless-Chromium click-through on `https://mixsets.syco23.org` at 1440×1000 and 390×844 — all indexes, a mix detail, `/review` and `/providers` rendered, no overflow, no console errors; see `docs/PRODUCTION_VERIFICATION.md`.)* If the browser capability is unavailable, report that fact and preserve a concrete mobile verification follow-up rather than claiming a runtime pass.
- [x] Recreate the worker and recheck stable catalog IDs, covers, waveforms and decisions. Verify a production backup in a separate test database without replacing live data. Record provider limitations accurately, including SoundCloud's missing secret and the rejected replacement Discogs token.
- [x] Commit the verification/docs changes, synchronize the authorized branch/deployment, and attach any pull request created or updated for this work to the task. *(Done 2026-10-05: docs committed and pushed to `master`; the backend deployment was synchronized to `582aea7`; no PR was required.)* Report the deployed result, evidence and remaining limitations.

## Plan self-review

The tasks cover the approved catalog, all five indexes, identity constraints, field evidence, provider capability limits, canonical protection, Review decisions, curator access, migration, persistent media, rollout and restoration. Shared names/types above are the contract between tasks. No task may replace the catalog's selected fields in the browser or treat search similarity as identity proof. Existing discovery queues may remain transient; canonical catalog data, review decisions and enrichment outcomes must persist.

Recommended execution: **Native**, because the nine tasks share schema/repository/merge interfaces and implementing them in one session avoids repeated interface handoffs. Use one independent whole-branch reviewer before production rollout, as required by the executing-plans workflow.
