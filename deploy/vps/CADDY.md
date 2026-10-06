# Shared VPS-L Caddy route

The shared proxy at `/opt/syco23-ftm/Caddyfile` routes both API hostnames to the
Mixsets worker on the `syco23-ftm_default` Docker network:

```caddyfile
mixsets-api.syco23.org, mixsets-api.87-106-219-4.sslip.io {
    import ftm_headers
    encode zstd gzip
    reverse_proxy syco23-mixsets-api:8787
}
```

Validate a changed Caddyfile with `docker exec syco23-ftm-caddy caddy validate
--config /etc/caddy/Caddyfile`. The proxy has `admin off`, so apply a validated
change with `docker compose restart caddy` from `/opt/syco23-ftm`.
