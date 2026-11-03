# Vercel-controlled app authentication

Vercel project: `syco23-mixsets`, SYCO23 team. Set `AUTH_MODE=off` to allow curator actions without a login. Set `VITE_AUTH_MODE` to the same value for the static frontend's login/logout presentation. The API session response reports its effective mode and determines whether curator actions are available.

The default is `on`; only the exact values `on` and `off` are accepted. `off` is a deliberate high-trust deployment mode: it gives every API caller curator rights, including non-browser clients. Write requests retain the configured exact-origin checks and input validation, but origin checks are not authentication. This flag does not change Vercel deployment protection.

The VPS does not automatically receive Vercel variables. Synchronize the current production export before each worker rollout:

```sh
vercel env pull /run/mixsets-vercel.env --environment=production --yes
python3 deploy/vps/sync_auth_mode.py /run/mixsets-vercel.env deploy/vps/.env
rm /run/mixsets-vercel.env
docker compose --env-file deploy/vps/.env -f deploy/vps/compose.yml up -d api
```

Run the Vercel CLI in the linked project context. MCP deployments may retrieve `AUTH_MODE` from Vercel directly and write a temporary export containing that one public configuration value; then invoke the same sync script. Provider credentials remain in the private VPS env and are never copied from the frontend or committed.

Rebuild/redeploy the Vercel frontend when changing `VITE_AUTH_MODE`. Restart/recreate the worker after synchronizing `AUTH_MODE`. Enabling `on` requires a privately configured `CURATOR_PASSWORD_HASH`; use `setup_curator.py` interactively first. The sync script preserves the existing private env, rejects invalid/duplicate source modes and replaces the env atomically with owner-only permissions. `CURATOR_AUTH_DISABLED` remains available for local development only and is ignored in production.

The Compose service mounts `syco23-mixsets_mixsets_data` at `/app/data`. Keep this volume when replacing the image. Use `backup_catalog.py` and `restore_catalog.py` for SQLite backups and restore verification.
