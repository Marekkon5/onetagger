# server/src/providers — provider adapters

## Purpose

Translate external provider responses into validated `MixCandidate`/metadata facts and artwork for discovery, enrichment and source refresh.

## Ownership

- One adapter per provider: `freeteknomusic.ts`, `soundcloud.ts`, `archiveorg.ts`, `discogs.ts`, `youtube.ts`, `hearthis.ts`, `mixcloud.ts`.
- `artwork.ts` — bounded provider-image fetching.
- `source-refresh.ts` — refreshing already-linked sources.

## Local Contracts

- Provider output is untrusted. Bound payload sizes, validate URLs and never copy arbitrary provider JSON into a canonical record.
- Preserve meaning: an uploader/channel is recorded as uploader, never promoted to performer; upload/creation time is not a recording date; avatars and generic logos are not cover art.
- Use documented public APIs/oEmbed. No HTML scraping, access-control bypass or protected-stream extraction.
- Keep credential-free paths credential-free (Mixcloud read API, hearthis.at, YouTube known-video oEmbed). Credentialed paths read secrets only from the server environment.
- Artwork must come from the provider's approved image hosts. Redirects are re-validated.
- Health states are `ready`, `limited` and `offline`; a rate limit is `limited`, not a silent empty success.
- Provider IDs and capabilities must match `PROVIDER_CAPABILITIES` in `packages/domain` and the worker resolvers.

## Work Guidance

- Add fixture-based tests for each new response shape and each rejection case (foreign host, avatar, upload date, short clip).
- Never log or return tokens, secrets or credential-bearing URLs.
- When a provider gains a new capability, update the domain matrix, the server adapter, the worker resolver and the docs together.

## Verification

- `pnpm --dir server exec tsx --test src/providers/*.test.ts`
- `pnpm --dir server test`
- `pnpm --dir server typecheck`

## Child DOX Index

None; each adapter is covered by this document.
