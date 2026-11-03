# server — Node API and catalog

## Purpose

Owns the running HTTP API, the SQLite-backed canonical catalog, curator authentication, provider adapters, enrichment, discovery/waveform queues and the optional durable import control plane.

## Ownership

- `server/src/` (API, catalog, auth, providers, imports, jobs, core).
- `server/src/catalog/migrations/` (ordered schema migrations).
- `server/Dockerfile` and server runtime configuration.
- `server/src/index.ts` is the composition root; route modules return `boolean` and own their own CORS/error handling.

## Local Contracts

- One catalog process per SQLite database. WAL, foreign keys and a bounded busy timeout are enabled; network and audio work happen outside short synchronous transactions.
- Public GET routes serve confirmed records. Curator writes require a session, an exact allowed origin and revision checks. Proposed records and unfinished runs stay hidden from public readers.
- Error responses are mapped through an explicit safe-message allow-list. Unrecognized failures are logged and returned as a generic 500.
- `CATALOG_PASSWORD_HASH`/`CURATOR_PASSWORD_HASH` is scrypt; sessions are hashed server-side and carried in an HTTP-only cookie. `CURATOR_AUTH_DISABLED` is local-only and ignored in production.
- `/api/imports` is curator-owned; `/internal/imports` is HMAC-SHA256 worker-owned. The import plane stays dark unless `IMPORTS_ENABLED` is set and a real curator hash exists.
- Discovery and waveform jobs are in-memory and single-worker; catalog records, claims, review decisions, media, enrichment runs and import jobs persist in SQLite.
- Migrations require both an ordered SQL file and an entry in the array in `database.ts`.

## Work Guidance

- Keep provider HTTP calls out of repository/merge transactions.
- Validate every untrusted body with a shared schema before it reaches the repository.
- When adding a route, state whether it is public, curator-only or worker-only, and test the denied path as well as the happy path.
- Keep CORS allow-headers aligned with headers the client actually sends (`content-type`, `idempotency-key`).
- Prefer additive migrations; do not edit an applied migration.

## Verification

- `pnpm --dir server test`
- `pnpm --dir server typecheck`
- `pnpm --dir server build`

## Child DOX Index

- **`src/catalog/`** — repository, identity, claims/merge, review, enrichment, migration, media, backup and catalog routes. Own AGENTS.md at `server/src/catalog/AGENTS.md`.
- **`src/imports/`** — durable import control plane, policy, worker authentication and private storage. Own AGENTS.md at `server/src/imports/AGENTS.md`.
- **`src/providers/`** — provider adapters, artwork fetching and source refresh. Own AGENTS.md at `server/src/providers/AGENTS.md`.
