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
- The worker never opens SQLite. It reports artifacts with a validated role, object key shape, SHA-256 and bounded size/duration.
- User uploads are a private `urn:syco23:upload:…` handle. Never store a filesystem path or put a URN into a canonical `ProviderRef`.
- Finalize must build the catalog record from real worker metadata/artifacts and apply claims that carry actual values. Placeholder titles and URL-valued title claims are defects, not acceptable fallbacks.
- `ImportJobDetail` evidence payloads use `hardGates`, matching the repository and domain schema. CORS must allow `idempotency-key` in preflight.

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
