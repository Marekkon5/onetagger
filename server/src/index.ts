import http from 'node:http';
import { URL } from 'node:url';
import { ProviderRegistry } from './core/registry.js';
import type { ProviderId } from './domain.js';
import { ArtworkFetchError, fetchProviderArtwork } from './providers/artwork.js';
import { InMemoryJobQueue, JobQueueFullError } from './jobs/in-memory.js';
import { enrichMix } from './core/enrichment.js';
import { WaveformQueue } from './core/waveform.js';
import { openCatalog } from './catalog/repository.js';
import { createCuratorAuth } from './auth/curator.js';
import { handleAuthRoute } from './auth/routes.js';
import { handleCatalogRoute } from './catalog/routes.js';
import { HttpInputError, isAllowedOrigin, publicErrorMessage, readJsonBody } from './http.js';
import { handleImportRoute } from './imports/routes.js';

const port = Number(process.env.PORT || 8787);
const catalog = openCatalog(process.env.CATALOG_DB_PATH || './data/catalog.sqlite');
// Runs left open by a previous process can never finish. Close them out at start so a
// record is never implicitly "enriching" forever and every interruption is retryable.
const interruptedRuns = catalog.interruptStaleEnrichmentRuns();
if (interruptedRuns > 0) console.log(`Closed ${interruptedRuns} interrupted enrichment run(s) from a previous process`);
const curatorAuth = createCuratorAuth(catalog, process.env.CURATOR_PASSWORD_HASH || '');
// Audit gate: import jobs create network/CPU/storage work. They stay dark until
// explicitly enabled AND real curator authentication is configured, so AUTH_MODE
// off/high-trust deployments can never expose the import control plane anonymously.
const curatorAuthConfigured = (process.env.CURATOR_PASSWORD_HASH || '').startsWith('scrypt$');
const importsEnabled = /^(1|true|yes|on)$/i.test(process.env.IMPORTS_ENABLED ?? '') && curatorAuthConfigured;
if (process.env.IMPORTS_ENABLED && !importsEnabled) {
  console.warn('IMPORTS_ENABLED was requested but curator authentication is not configured; imports stay disabled');
}
const registry = new ProviderRegistry();
const queue = new InMemoryJobQueue(registry);
const waveformQueue = new WaveformQueue();

function corsOrigin(req: http.IncomingMessage): string | undefined {
  const requested = req.headers.origin;
  return requested && isAllowedOrigin(req) ? requested : undefined;
}

function json(req: http.IncomingMessage, res: http.ServerResponse, status: number, body: unknown) {
  const payload = status === 204 ? '' : JSON.stringify(body);
  const origin = corsOrigin(req);
  res.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    ...(status === 204 ? {} : { 'content-length': Buffer.byteLength(payload) }),
    ...(origin ? {
      'access-control-allow-origin': origin,
      'access-control-allow-credentials': 'true',
      'access-control-allow-headers': 'content-type',
      'access-control-allow-methods': 'GET,POST,PATCH,PUT,OPTIONS',
      vary: 'Origin'
    } : {}),
  });
  res.end(payload);
}

async function body(req: http.IncomingMessage): Promise<any> {
  return readJsonBody(req);
}

const server = http.createServer(async (req, res) => {
  if (!req.url) return json(req, res, 404, { error: 'not found' });
  if (req.method === 'OPTIONS') return json(req, res, 204, {});
  const url = new URL(req.url, `http://${req.headers.host || 'localhost'}`);
    if (await handleAuthRoute({ auth: curatorAuth }, req, res)) return;
    if (await handleCatalogRoute({ repository: catalog, auth: curatorAuth, registry }, req, res)) return;
    // Import control plane handles its own auth: curator session for /api/imports,
    // HMAC service identity for /internal/imports. Handled before the generic
    // curator gate so the worker's signed requests are not rejected for a cookie.
    if (await handleImportRoute({ repository: catalog, auth: curatorAuth, importsEnabled }, req, res)) return;

    const curatorRead = req.method === 'GET' && (url.pathname.startsWith('/api/jobs') || url.pathname.startsWith('/api/waveforms'));
    if (req.method === 'POST' || req.method === 'PUT') {
      if (!isAllowedOrigin(req)) return json(req, res, 403, { error: 'Origin is not allowed' });
      if (!curatorAuth.authenticate(req)) return json(req, res, 401, { error: 'Curator login is required' });
    } else if (curatorRead && !curatorAuth.authenticate(req)) {
      return json(req, res, 401, { error: 'Curator login is required' });
    }

  try {
    if (req.method === 'GET' && url.pathname === '/api/live') {
      return json(req, res, 200, { ok: true, service: 'syco23-mixsets' });
    }
    if (req.method === 'GET' && url.pathname === '/api/health') {
      return json(req, res, 200, { ok: true, service: 'syco23-mixsets', providers: await registry.health() });
    }
    if (req.method === 'GET' && url.pathname === '/api/providers') {
      return json(req, res, 200, await registry.health());
    }
    if (req.method === 'GET' && url.pathname === '/api/jobs') {
      return json(req, res, 200, queue.list());
    }
    if (req.method === 'POST' && url.pathname === '/api/jobs') {
      const input = await body(req);
      if (!input.provider || (!input.query?.q && !input.query?.url && !input.query?.artist)) return json(req, res, 400, { error: 'provider and query are required' });
      try {
        return json(req, res, 202, queue.create(String(input.provider), input.query));
      } catch (error) {
        if (error instanceof JobQueueFullError) return json(req, res, 429, { error: error.message });
        throw error;
      }
    }
    const jobMatch = url.pathname.match(/^\/api\/jobs\/([^/]+)$/);
    if (req.method === 'GET' && jobMatch) {
      const job = queue.get(jobMatch[1]);
      return job ? json(req, res, 200, job) : json(req, res, 404, { error: 'job not found' });
    }
    const cancelMatch = url.pathname.match(/^\/api\/jobs\/([^/]+)\/cancel$/);
    if (req.method === 'POST' && cancelMatch) {
      const job = queue.cancel(cancelMatch[1]);
      return job ? json(req, res, 200, job) : json(req, res, 404, { error: 'job not found' });
    }
    if (req.method === 'POST' && url.pathname === '/api/enrich') {
      const input = await body(req);
      if (!input.title || !Array.isArray(input.artists)) return json(req, res, 400, { error: 'title and artists are required' });
      const providerIds: ProviderId[] = ['freeteknomusic', 'soundcloud', 'archiveorg', 'discogs', 'youtube', 'hearthis'];
      if (input.providers !== undefined && (!Array.isArray(input.providers) || input.providers.some((provider: unknown) => !providerIds.includes(provider as ProviderId)))) {
        return json(req, res, 400, { error: 'providers must contain only supported provider IDs' });
      }
      return json(req, res, 200, await enrichMix(registry, {
        ...input,
        providers: input.providers as ProviderId[] | undefined,
      }));
    }
    if (req.method === 'POST' && url.pathname === '/api/artwork/fetch') {
      const input = await body(req);
      if (typeof input.url !== 'string' || input.url.length > 2048) {
        return json(req, res, 400, { error: 'A provider artwork URL is required' });
      }
      try {
        const image = await fetchProviderArtwork(input.url);
        const origin = corsOrigin(req);
        res.writeHead(200, {
          'content-type': image.contentType,
          'content-length': image.bytes.byteLength,
          'cache-control': 'private, no-store',
          'x-content-type-options': 'nosniff',
          ...(origin ? {
            'access-control-allow-origin': origin,
            'access-control-allow-credentials': 'true',
            vary: 'Origin',
          } : {}),
        });
        res.end(image.bytes);
        return;
      } catch (error) {
        const status = error instanceof ArtworkFetchError ? error.status : 502;
        return json(req, res, status, { error: error instanceof Error ? error.message : 'Provider artwork fetch failed' });
      }
    }
    if (req.method === 'POST' && url.pathname === '/api/waveforms') {
      const input = await body(req);
      if (typeof input.sourceUrl !== 'string' || input.sourceUrl.length > 2048) return json(req, res, 400, { error: 'A direct public audio URL is required' });
      try { return json(req, res, 202, waveformQueue.create(input.sourceUrl)); }
      catch (error) { return json(req, res, 422, { error: error instanceof Error ? error.message : String(error) }); }
    }
    const waveformMatch = url.pathname.match(/^\/api\/waveforms\/([^/]+)$/);
    if (req.method === 'GET' && waveformMatch) {
      const job = waveformQueue.get(waveformMatch[1]);
      return job ? json(req, res, 200, job) : json(req, res, 404, { error: 'waveform job not found' });
    }
    if (req.method === 'POST' && url.pathname === '/api/soundcloud/artwork') {
      const input = await body(req);
      if (typeof input.url !== 'string' || input.url.length > 2048) return json(req, res, 400, { error: 'A public SoundCloud track URL is required' });
      try {
        const artwork = await registry.soundcloud.resolvePublicArtwork(input.url);
        return artwork.artworkUrl
          ? json(req, res, 200, artwork)
          : json(req, res, 422, { error: 'That SoundCloud link has no track cover artwork' });
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return json(req, res, message.includes('URL is invalid') ? 400 : 502, { error: message });
      }
    }
    if (req.method === 'POST' && url.pathname === '/api/artwork/preview') {
      const input = await body(req);
      if (typeof input.url !== 'string' || input.url.length > 2048 || !['soundcloud', 'youtube', 'hearthis'].includes(input.provider)) {
        return json(req, res, 400, { error: 'Choose SoundCloud, YouTube, or hearthis.at and enter a public track URL' });
      }
      try {
        if (input.provider === 'soundcloud') {
          const artwork = await registry.soundcloud.resolvePublicArtwork(input.url);
          return artwork.artworkUrl ? json(req, res, 200, { ...artwork, provider: 'soundcloud' })
            : json(req, res, 422, { error: 'That SoundCloud link has no track cover artwork' });
        }
        if (input.provider === 'youtube') {
          const video = await registry.youtube.lookupVideo(input.url);
          return video.artworkUrl ? json(req, res, 200, { provider: 'youtube', sourceUrl: `https://www.youtube.com/watch?v=${video.id}`, title: video.title, artworkUrl: video.artworkUrl })
            : json(req, res, 422, { error: 'That YouTube video has no public thumbnail' });
        }
        const track = await registry.hearthis.lookupTrack(input.url);
        const artworkUrl = await registry.hearthis.lookupArtwork(input.url);
        return artworkUrl ? json(req, res, 200, { provider: 'hearthis', sourceUrl: track.permalink_url, title: track.title, artworkUrl })
          : json(req, res, 422, { error: 'That hearthis.at track has no cover artwork' });
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return json(req, res, message.includes('URL is invalid') ? 400 : 502, { error: message });
      }
    }
    if (req.method === 'POST' && url.pathname === '/api/discogs/enrich') {
      const input = await body(req);
      if (!input.name) return json(req, res, 400, { error: 'name is required' });
      const kind = ['artist', 'crew', 'label'].includes(input.kind) ? input.kind : 'artist';
      return json(req, res, 200, await registry.discogs.enrichEntity(String(input.name), kind));
    }
    return json(req, res, 404, { error: 'not found' });
  } catch (error) {
    const status = error instanceof HttpInputError ? error.statusCode : 500;
    return json(req, res, status, { error: publicErrorMessage(error, 'Request failed') });
  }
});

server.listen(port, '0.0.0.0', () => {
  console.log(`SYCO23 Mixsets API listening on :${port}`);
});

for (const signal of ['SIGINT', 'SIGTERM'] as const) {
  process.once(signal, () => server.close(() => catalog.close()));
}
