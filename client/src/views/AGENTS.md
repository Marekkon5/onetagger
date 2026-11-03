# client/src/views — route screens

## Purpose

Route-level screens for browsing the five indexes, opening mix/entity/event details, importing and enriching, reviewing conflicts, merging duplicates, tagging local files and configuring providers.

## Ownership

- `CatalogIndexView.vue`, `CatalogDetailView.vue` — shared index/detail screens.
- `ImportView.vue`, `ReviewView.vue`, `ProvidersView.vue`, `LocalTaggerView.vue`, `CuratorLoginView.vue`.
- Shared visual components live in `client/src/components/`.

## Local Contracts

- Public views must not render curator editors or mutation controls. Curator-only actions handle 401 by redirecting to `/login?redirect=…`.
- Use `presentation.ts` for state, provider, field and value labels. Never show raw enums or internal JSON.
- Import flow: discovery creates reviewable candidates; indexing uses the shared catalog import; durable jobs use `importJobsApi`. Polling must stop on a parse/auth failure and must not silently present stale state.
- Do not ask for a curator password in a browser prompt when the server authenticates by session cookie. Finalize is a session-authorized action.
- Local tagger: never modify originals; write tagged copies to the separate output folder; report unsynced files honestly.
- Detail views must distinguish missing, unavailable, disputed and curator-confirmed values.

## Work Guidance

- Prefer a small `data-testid` for stable e2e hooks over brittle text selectors.
- Keep desktop and narrow layouts usable; check horizontal overflow.
- When adding a route, update `client/src/scripts/router.ts` and the primary navigation.
- Test loading, empty, error and unauthenticated states, not only the happy path.

## Verification

- `pnpm --dir client test`
- `pnpm --dir client exec vue-tsc --noEmit`
- `pnpm test:e2e`

## Child DOX Index

None; shared behavior is documented here and in `client/src/catalog/AGENTS.md`.
