# client — Vue 3/Quasar archive UI

## Purpose

The public reader and curator control surface. It browses the shared catalog, runs curator imports/enrichment/review/merge, and scans/tags local MP3s without modifying originals.

## Ownership

- `client/src/` (views, catalog transport/store, components, local MP3 pipeline, presentation layer).
- `client/vite.config.ts`, `client/tsconfig*.json`, `client/vercel.json`.
- Build output is generated; do not edit `client/dist/`.

## Local Contracts

- The server catalog is authoritative. A mutation updates UI state only after the API returns the committed record; a failed or offline write must never look saved.
- All API responses are parsed with shared Zod schemas. A malformed response fails visibly; do not silently fall back to a permissive shape.
- `credentials: 'include'` on transport calls. Curator-only actions redirect to `/login` on 401.
- Every machine identifier shown to a person goes through `client/src/catalog/presentation.ts`; never render raw enums, field keys, JSON or internal timestamps.
- The browser's legacy `syco23.mixsets.library` is read-only migration input. The app never deletes it.
- Local MP3 tagging writes normalized copies into a separate `Mixsets-tagged` folder and leaves originals untouched.
- Keep client contract types aligned with the server (`ImportJobDetail` evidence uses `hardGates`; finalize returns `{ record, mixId, claimsApplied }`).

## Work Guidance

- Add store/component tests for loading, failure, stale writes and malformed payloads.
- Keep public and curator modes distinct: hide editors and mutation controls from an unauthenticated reader.
- Use accessible labels and controls; verify narrow layouts without horizontal overflow.
- Prefer presentation helpers over per-view label maps.

## Verification

- `pnpm --dir client test`
- `pnpm --dir client exec vue-tsc --noEmit`
- `pnpm --dir client build`
- `pnpm test:e2e` for route-level behavior.

## Child DOX Index

- **`src/catalog/`** — API transport, store, view projections, presentation and migration. Own AGENTS.md at `client/src/catalog/AGENTS.md`.
- **`src/views/`** — route-level screens. Own AGENTS.md at `client/src/views/AGENTS.md`.
