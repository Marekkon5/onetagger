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
- The API mounts `mixsets_data:/app/data`; the worker mounts `mixsets_imports:/app/data/imports`. Both run read-only with dropped capabilities, `no-new-privileges`, pid/memory/CPU limits and log rotation.
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
