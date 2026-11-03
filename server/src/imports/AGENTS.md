# server/src/imports — durable import control plane

## Purpose

Owns the opt-in metadata/audio import plane: curator job creation, policy decisions, worker leases, worker-signed progress/artifacts/evidence/completion, private object storage and job finalization into the catalog.

## Ownership

- `routes.ts` — `/api/imports` and `/internal/imports` HTTP surface.
- `policy.ts` — request parsing and capability/rights policy.
- `service-auth.ts` — HMAC-SHA256 worker identity.
- `storage.ts` — private object-store keys, hashing and validation.

## Local Contracts

- The plane is dark unless `IMPORTS_ENABLED` is truthy **and** a real `scrypt$…` curator hash exists. It is not a public acquisition endpoint.
- Curator routes require an allowed origin and a curator session. Worker routes require a timestamp within tolerance, scope `import_worker`, and an HMAC over `timestamp\nmethod\npath\nbody`.
- `Idempotency-Key` is mandatory on job creation. The outbox row is the durable queue; a lease is 30 s and must be renewed by heartbeat.
- Rights are deployment-wide: `IMPORT_GLOBAL_RIGHTS_CONFIRMATION` defaults to true, and `resolveRightsPolicy` injects the `global_rights_confirmation` basis for every request so a client cannot narrow or override it. When the policy is disabled, a request must carry its own basis.
- Capabilities come from the shared `PROVIDER_CAPABILITIES`; SoundCloud/hearthis.at/Mixcloud audio is allowed only through provider-authorised downloads and may fail per item.
- The worker never opens SQLite. It reports artifacts with a validated role, object key shape, SHA-256 and bounded size/duration.
- Every worker mutation (events, artifacts, evidence, completion, heartbeat) requires the `x-import-worker-id` that holds an unexpired lease; a non-owner receives `409`.
- User uploads are a private `urn:syco23:upload:…` handle. Never store a filesystem path or put a URN into a canonical `ProviderRef`. A finalized upload has an empty `sources` array and carries the URN only in the job/provenance tables.
- Finalize must build the catalog record from real worker metadata/artifacts and apply claims that carry actual values. Placeholder titles and URL-valued title claims are defects, not acceptable fallbacks. Provider jobs require a readable, hash-checked `ImportMetadataFactsSchema` snapshot (`409` otherwise).
- `ImportJobDetail` evidence payloads use `hardGates`, matching the repository and domain schema. CORS must allow `idempotency-key` in preflight.
- The API and worker share `IMPORT_STORAGE_LOCAL_ROOT`/S3 configuration and the `mixsets_imports` volume. S3 requests use AWS Signature V4 with `IMPORT_S3_REGION`.

## Work Guidance

- Add a regression test for every provider path in the worker resolver before expanding the capability matrix.
- Test user-upload finalize end to end, including `CatalogRecordSchema` validation.
- Enforce lease ownership on every worker mutation, not only `heartbeat`.
- Bound request bodies before buffering; keep API/worker memory limits consistent with `compose.yml`.
- Distinguish `claimsApplied` (actually selected/corroborated) from the number of claims considered.

## Verification

- `pnpm --dir server exec tsx --test src/imports/*.test.ts`
- `pnpm --dir server test`
- `pnpm --dir server typecheck`

## Child DOX Index

None; the plane is documented here.
