# packages/domain — shared catalog contracts

## Purpose

The single source of typed truth shared by the client, server and worker: canonical catalog records, provider references and claims, import-job contracts, normalization, field definitions, completeness, evidence scoring and import policy.

## Ownership

- All files under `packages/domain/src/`.
- `index.ts` is the supported public surface; consumers import from `@syco23/catalog-domain`.
- This package owns contract shape, not HTTP, SQLite or provider transport.

## Local Contracts

- Every canonical record, provider claim, import job and API page has a strict Zod schema. Unknown keys are rejected unless a schema explicitly allows them.
- `ProviderIdSchema` includes `freeteknomusic`, `soundcloud`, `archiveorg`, `discogs`, `youtube`, `hearthis`, `mixcloud`. `ImportProviderSchema` adds `user_upload`.
- `ImportSourceUrlSchema` accepts an http(s) URL or a `urn:syco23:upload:…` handle. Canonical `ProviderRefSchema` and `FieldClaimSchema.sourceUrl` accept only http(s). Do not put a private URN into a canonical `ProviderRef`.
- `PROVIDER_CAPABILITIES` and `ProviderCapabilityMatrixSchema` must stay in sync with the adapters and the import worker. Every import provider needs an explicit metadata/audio entry.
- `ScoredEvidenceSchema` uses `hardGates` (the API field name). `scoreEvidence` is the deterministic `evidence-v1` implementation; gates fail closed.
- `decideImportPolicy` is the authoritative metadata/audio policy. Unknown providers default to no capability; audio requires an explicit non-metadata rights basis.
- Display spelling survives normalization. Normalize only search/identity keys; do not fold away meaningful tokens such as `live`, `set`, `dj`, `crew` or `live`.
- `FieldDefinitions` is the allow-list for claims and curated edits. A worker evidence field that is not in this map is ignored by merge policy.

## Work Guidance

- Add or change a schema with a test in `schemas.test.ts` or `import-jobs.test.ts`.
- Keep date precision (`year | month | day`), separate recording/upload/event dates, distinct asset roles and ISO country validation.
- When a caller needs a value the schema forbids, fix the caller or introduce a clearly named contract; never relax `strict()` casually.
- Build the package before typechecking or testing consumers: `pnpm --dir packages/domain build`.

## Verification

- `pnpm --dir packages/domain test`
- `pnpm --dir packages/domain typecheck`
- `pnpm --dir packages/domain build`

## Child DOX Index

None; the package is small enough for this document.
