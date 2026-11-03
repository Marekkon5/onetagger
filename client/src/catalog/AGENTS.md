# client/src/catalog — transport, store and presentation

## Purpose

Owns the typed catalog boundary for the browser: HTTP transport, authoritative read/mutation state, view projections, migration conversion and human-readable presentation maps.

## Ownership

- `api.ts` — `catalogApi`, `importJobsApi` and response schemas.
- `store.ts` — `useCatalogStore` state and committed-record updates.
- `views.ts` — canonical record to view-model projections.
- `presentation.ts` — labels, values, timestamps and state pills.
- `*.test.ts` — transport, store, projection, review and presentation tests.

## Local Contracts

- Parse every response with the shared domain schema. `ImportJobDetailSchema` must use `hardGates`, the field the server and repository emit.
- `catalogApi.finalize` must parse the server's `{ record, mixId, claimsApplied }` envelope, not a bare `CatalogRecord`. Keep it consistent with `services/api.ts#finalizeImportJob`.
- A mutation returns the committed record; the store replaces local state only after that response. Do not optimistically mark writes committed.
- Migration conversion is idempotent by batch id and never removes the browser's originals.
- `presentation.ts` is the only place that maps enum/state identifiers to human wording. It must not fall back to `JSON.stringify` for display.
- Field evidence parsing validates id, fingerprint, disposition and the embedded claim.

## Work Guidance

- When the server response shape changes, update the client schema and its test in the same change.
- Do not swallow parse errors in polling or background refreshes; surface them or stop with an honest state.
- Keep the store a cache/reader: no second canonical merge engine.
- Add a malformed-response test for every new schema.

## Verification

- `pnpm --dir client test`
- `pnpm --dir client exec vue-tsc --noEmit`

## Child DOX Index

None; presentation and transport are documented here.
