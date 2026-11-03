# worker — stateless import data plane

## Purpose

Leases durable import jobs from the API, resolves and bounds provider media, transcodes audio with FFmpeg, writes private artifacts and reports evidence/completion. It never opens SQLite.

## Ownership

- `worker/src/index.ts` — poll loop and shutdown.
- `queue.ts` — claim, classify, process and complete one job.
- `control.ts` — HMAC-signed control-plane client.
- `resolvers.ts` — per-provider source resolution.
- `security.ts` — URL/DNS/redirect/size guards.
- `ffmpeg.ts`, `process.ts` — probe, convert, read-back verification, evidence.
- `storage.ts`, `config.ts` — private object store and environment configuration.

## Local Contracts

- The worker only talks to `/internal/imports` with the canonical HMAC and `x-import-worker-id`. It never reads the catalog database or serves public traffic.
- `resolveSource` must have a case for every provider the capability matrix advertises. An advertised-but-unresolvable provider is a control-plane defect.
- Audio uses provider-authorised mechanisms only: SoundCloud API `download_url` for downloadable tracks, hearthis.at `download_url` or its official listen `stream_url`, and Mixcloud's download route only when Mixcloud reports the show downloadable. Never decode an obfuscated stream or extract a protected one; fail non-retryably instead.
- Discogs and MusicBrainz resolvers send a descriptive User-Agent; Discogs may use `DISCOGS_TOKEN` and SoundCloud uses `SOUNDCLOUD_CLIENT_ID`/`SECRET` or `SOUNDCLOUD_ACCESS_TOKEN`.
- Every fetch is bounded: scheme/host/port validation, DNS resolution checks, redirect re-validation, byte cap and timeout. Private/loopback/link-local/metadata addresses are rejected.
- Object keys are derived only from job id, role and content hash. Never use an original filename or path.
- Audio jobs heartbeat the lease, stop on lease loss, probe duration, convert to 320 kbps MP3, re-probe and reject >2% drift (with a 2 s tolerance).
- Metadata and audio jobs must snapshot the full sourced facts (`ImportMetadataFactsSchema`, not only title/artwork) so finalize can create a real record, and submit per-field evidence carrying the sourced `value`.
- Keep configured limits within the deployed container: API 768 MB, worker 1.5 GB, `/tmp` tmpfs 640 MB. Input and output quotas default to 256 MiB each.
- S3-compatible storage is signed with AWS Signature V4 (`IMPORT_S3_REGION`); never send unsigned private-object requests.
- The worker and API must share the same `IMPORT_STORAGE_LOCAL_ROOT`/S3 configuration and the `mixsets_imports` volume, otherwise finalize cannot read the metadata snapshot.

## Work Guidance

- Add a resolver + fixture test before adding a provider to `PROVIDER_CAPABILITIES`.
- Keep classification honest: `NonRetryableError` for policy/media/URL rejection, `RetryableError` for transient transport failures. `boundedFetch` raises `UpstreamHttpError` for a non-2xx provider response; `408`/`425`/`429`/`5xx` retry and other `4xx` statuses fail permanently.
- Never log secrets or signed headers.
- Verify read-back output, not just the FFmpeg exit code.

## Verification

- `pnpm --dir worker test`
- `pnpm --dir worker typecheck`
- `pnpm --dir worker build`

## Child DOX Index

None; the worker is a single module set.
