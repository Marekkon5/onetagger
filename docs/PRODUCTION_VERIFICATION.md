# Production Verification

Date: 2026-10-04 (local checks re-run; browser and VPS findings unchanged from 2026-10-03)

## Local implementation checks

- `pnpm typecheck`: passed for shared domain, Vue client and Node server.
- `pnpm test`: passed, **108 tests total (5 domain, 24 client, 75 server, 4 deployment)**. The deployment suite covers the private provider-setup routine: atomic secret-file replacement, duplicate secret keys, rejection of unsafe `.env` characters and owner-only permissions.
- `pnpm build`: passed for client and server. Rollup reports removable `@__PURE__` comment-position warnings from Zod; output is produced successfully.

Coverage now includes, beyond the earlier schema / provider / migration / media / backup set:

- **Catalog error classification** — reachable domain failures keep their HTTP status (404 missing record, 409 revision conflict, 400 bad request), while an internal fault returns 500 with a generic body and never echoes paths, SQL or secrets.
- **Durable enrichment runs** — a run row is written before provider work starts, so a crashed pass is recorded `interrupted` and stays retryable. Worker start closes runs left `running` by a previous process. A provider failure still completes the run, so valid results from other providers are not discarded.
- **Duplicate merge** — source identities, claims, review decisions, media and relationships move to the survivor. A field disagreement becomes a Review item rather than being resolved by write order. The retired id keeps resolving, and a stale second curator session is refused.
- **Field evidence** — the claim behind a selected value is retrievable and public wherever the record itself is public; a proposed record's evidence stays hidden.
- **Inbound entity navigation** — an entity page lists the mixes that reference it, including mixes that arrived through import or enrichment rather than legacy migration.
- **Browser-local state** — `useMixStore` holds provider health and job progress only, persists nothing, and cannot make an offline or failed write look committed.
- **Provider fan-out bounds** — the discovery queue caps concurrency and answers 429 instead of accepting unbounded work.
- **Search normalization** — `normalizeQuery` preserves `live`, `set` and `dj`, because those distinguish one recording from another ("Live at Wacken", "DJ Koalisson"). Only the `mix` family is dropped, and only in trailing position.

Smoke tests below were run on 2026-10-03 and were **not repeated** after the 2026-10-04 changes:

- Browser smoke test: local public catalog, curator login, Archive.org discovery and the known public Kan10/Mackitek YouTube video resolved. A low-confidence Archive.org result was not imported. The selected YouTube source created a shared detail and provider claims; the matching cover filled, alternative provider values remained in Review, and the interface reported remaining fields and provider errors.
- Local tagger smoke test: `/local-tags` loaded on the fresh Vite server, reported six provider states, required curator login for enrichment, and confirmed browser folder-picker support. No real user folder was selected; MP3 read/write is covered by unit tests.
- Desktop and 320 CSS-pixel mobile checks: no horizontal overflow at tested widths. A 312 CSS-pixel emulation is below the app's existing 320-pixel minimum and overflows by 8 pixels.

**Open item, not a pass:** the duplicate-merge comparison view, the field-evidence panel and the inbound-mix list on an entity page have unit and HTTP-level coverage but have not been clicked through in a running browser since these changes landed.

## Production status

Deployment was requested and the VPS was checked read-only through `ssh ionos_vps_l`. At verification time the API was healthy at commit `7368973`, with Discogs ready. **No deployment has been performed.** The deployed commit is still `7368973`, so the catalog work committed after it is not live.

Unchanged blocking prerequisites:

- the private VPS `.env` has no `CURATOR_PASSWORD_HASH`;
- the current API container has no mounted data volume;
- the proposed Compose configuration requires the curator hash and introduces the persistent catalog volume.

Replacing the live worker before privately configuring the curator and confirming the catalog and backup plan would be unsafe. The Vercel frontend was not deployed.

Before rollout: set `CURATOR_PASSWORD_HASH` privately with `deploy/vps/setup_curator.py`, deploy the pinned Node 22.23.3 image and the persistent `/app/data` volume, verify public reads / authenticated writes / exact-origin CORS, perform and verify an online backup, deploy the frontend, migrate each real browser library, and re-check after worker recreation. The discovery and waveform job queues remain in memory and require a single worker.

Provider constraints remain: SoundCloud official search needs its client secret or a supported access token; YouTube text search needs its API key. A provider failure leaves fields missing and is reported as such — it is never presented as "nothing new to add".