# deploy/vps — VPS deployment and operations

## Purpose

Deploy the API and optional import worker, configure the curator privately, persist the catalog/media volumes, and provide verified backup/restore and catalog bundle tooling.

## Ownership

- `compose.yml` — API and import-worker services, volumes, hardening and healthchecks.
- `.env.example` — documented deployment variables (never a real secret).
- `setup_curator.py`, `setup_providers.py`, `sync_auth_mode.py` — private interactive setup.
- `backup_catalog.py`, `restore_catalog.py`, `catalog_bundle.py` — consistent snapshot/restore.
- `test_*.py` — deployment tests.
- `AUTH_MODE.md`, `CADDY.md` — operating notes.

## Local Contracts

- Real secrets live only in `deploy/vps/.env` with owner-only permissions. Never commit them or paste them into the browser.
- The API mounts `mixsets_data:/app/data` and `mixsets_imports:/app/data/imports`; the worker mounts the same `mixsets_imports` volume. Both run read-only with dropped capabilities, `no-new-privileges`, pid/memory/CPU limits and log rotation.
- The API and worker must receive identical import storage configuration (`IMPORT_STORAGE_DRIVER`, local root or S3 endpoint/bucket/keys/region) so finalize can read worker artifacts and the worker can read accepted originals.
- `IMPORT_GLOBAL_RIGHTS_CONFIRMATION` defaults to `true`: the operator attests once that they hold the rights to download/process provider content, and the API stores that basis for every import. Set it to `false` only to require an explicit per-request basis.
- The worker receives the provider credentials it needs for import (`SOUNDCLOUD_*`, `DISCOGS_TOKEN`, `MUSICBRAINZ_USER_AGENT`) from the same private environment; never expose them to the client bundle.
- Import quotas default to 256 MiB input and output, sized for the API 768 MB and worker 1.5 GB / 640 MB `/tmp` limits.
- `AUTH_MODE=off` is an explicit open-access deployment. It must not be combined with an enabled import plane; import acquisition must stay curator-gated.
- Backups use SQLite's online backup/VACUUM INTO over a read-write connection, verify integrity and foreign keys, and restore only to a new path. A live database file copy alone is not a backup.
- Deployment scripts validate inputs and never echo credentials or hashes.
- Production Node runtime is pinned to 22.23.3; pnpm to 12.8.1.

## Work Guidance

- Keep the Python tooling standard-library-only unless a dependency is unavoidable and documented.
- Any new environment variable belongs in `.env.example`, `compose.yml` and the relevant doc.
- Test scripts with `unittest`; prefer a temporary directory and never touch the live catalog.
- After a release, record dated evidence in `docs/PRODUCTION_VERIFICATION.md`; do not rewrite older records.

## Verification

- `pnpm test:deploy` (or `pnpm test:deployment`)
- `docker compose -f deploy/vps/compose.yml config` when changing Compose.

## Child DOX Index

None; all deployment tooling is covered here.
