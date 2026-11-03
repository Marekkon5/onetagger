# server/src/catalog — canonical persistence and curation

## Purpose

Owns the canonical catalog: records, provider identities, typed relationships, claims, review decisions, sessions, migration batches, media references, enrichment/analysis runs and the normalized read projection.

## Ownership

- `database.ts`, `repository.ts` — SQLite schema access and transactions.
- `identity.ts`, `merge.ts` — identity resolution and the one merge policy.
- `enrich.ts`, `provider-claims.ts` — provider facts to typed claims.
- `migration.ts`, `media.ts`, `backup.ts` — legacy migration, waveform assets and backup.
- `routes.ts` — catalog HTTP surface.
- `migrations/*.sql` — ordered schema changes.

## Local Contracts

- `repository.ts` is the only module that opens or writes SQLite. `saveRecord` validates through `CatalogRecordSchema`; do not bypass it.
- A provider identity is `(provider, resourceType, externalId)`. Repeated source imports are idempotent. Equal external IDs in different namespaces stay distinct.
- One merge policy in `merge.ts`: direct/curated claims may fill missing fields; equivalent values corroborate; disagreements preserve the selected value and create Review; parsed facts wait for a curator.
- Claim fingerprints are stable. Re-running enrichment must not duplicate pending items or resurrect rejected claims.
- A selected field stores the claim id in `selectedEvidence`; the claim remains retrievable for public readers wherever the record is public.
- Duplicate merge is transactional, revision-checked and keeps disagreeing fields in Review. The retired id becomes a legacy alias.
- Review acceptance checks the current record revision; stale decisions fail rather than overwrite newer curation.
- Enrichment runs are written before provider work and closed as `interrupted` on restart; a provider failure does not roll back other providers.
- Public routes hide `proposed` records and unfinished runs. Curator mutations must check origin, session and revision.
- There must be exactly one handler per method+path. A second `POST /api/catalog/merge` is unreachable and must not be added.

## Work Guidance

- Add tests for identity, merge, review revisions, completeness and migration before changing those behaviors.
- Keep `hardGates` as the evidence field name; do not introduce a second spelling in route payloads.
- When a worker or import job produces evidence, map real values into `FieldClaim`; do not infer a title from a URL.
- Curator attribution must describe the curator, not borrow a real provider namespace.
- Network and audio work stay outside transactions; transactions stay synchronous and short.

## Verification

- `pnpm --dir server exec tsx --test src/catalog/*.test.ts`
- `pnpm --dir server test`
- `pnpm --dir server typecheck`

## Child DOX Index

None; catalog details stay in this document.
