# tests — end-to-end verification

## Purpose

Black-box Playwright click-throughs against the real Vite client, real catalog API and real SQLite database. Only external provider HTTP is replaced with a deterministic fixture.

## Ownership

- `e2e/catalog.spec.ts` — catalog browsing, curator edits, review, merge, migration, stale-save and import flows.
- `e2e/local-tagger.spec.ts` — browser-local MP3 auto-tagging.
- `e2e/serve-api.mts` — disposable API instance, fixture records and provider stubs.

## Local Contracts

- The provider boundary is stubbed; catalog, auth, routes, SQLite, migration and media stay real.
- Use a fresh temporary database per run. Never point e2e at production or a developer's catalog.
- Wait for a committed state before capturing an id or URL. After clicking a navigation link, wait for the URL or a detail-only test id; a candidate heading on the previous page is not proof of navigation.
- Assert public visibility from a fresh browser context, not the curator session.
- Don't assert timing-sensitive counts unless the fixture makes them deterministic.
- Screenshots/traces go under `/tmp/syco23-catalogue-verification` by default.

## Work Guidance

- Prefer role/label/test-id selectors that match the accessible UI.
- When a test exposes a race, fix the wait condition rather than adding arbitrary sleeps.
- Keep fixture providers offline except for the one path under test.
- Add a case for a new public index, a new curator mutation and a new import/job state.

## Verification

- `pnpm test:e2e`
- `pnpm exec playwright test tests/e2e/catalog.spec.ts:<line>` for a focused run.

## Child DOX Index

None; the suite is small and fixture-driven.
