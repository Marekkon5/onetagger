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
- Every fetch is bounded: scheme/host/port validation, DNS resolution checks, redirect re-validation, byte cap and timeout. Private/loopback/link-local/metadata addresses are rejected.
- Object keys are derived only from job id, role and content hash. Never use an original filename or path.
- Audio jobs heartbeat the lease, stop on lease loss, probe duration, convert to 320 kbps MP3, re-probe and reject >2% drift (with a 2 s tolerance).
- Metadata jobs must snapshot the full sourced facts (not only title/artwork) so finalize can create a real record.
- Keep configured limits within the deployed container: API 512 MB, worker 1 GB, `/tmp` tmpfs 256 MB. A 2 GiB input limit is not usable in that shape.
- S3 mode currently supplies credentials but the client does not sign requests; do not treat it as production-ready until signing is implemented.

## Work Guidance

- Add a resolver + fixture test before adding a provider to `PROVIDER_CAPABILITIES`.
- Keep classification honest: `NonRetryableError` for policy/media/URL rejection, `RetryableError` for transient transport failures.
- Never log secrets or signed headers.
- Verify read-back output, not just the FFmpeg exit code.

## Verification

- `pnpm --dir worker test`
- `pnpm --dir worker typecheck`
- `pnpm --dir worker build`

## Child DOX Index

None; the worker is a single module set.
