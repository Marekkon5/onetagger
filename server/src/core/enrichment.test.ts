import assert from 'node:assert/strict';
import test from 'node:test';
import type { ProviderRegistry } from './registry.js';
import { enrichMix } from './enrichment.js';

test('enrichMix fills only missing fields and keeps conflicting identity as review material', async () => {
  const archive = {
    id: 'archiveorg' as const,
    async health() {
      return { id: 'archiveorg' as const, state: 'ready' as const, detail: 'ok', checkedAt: new Date().toISOString() };
    },
    async search() {
      return [{
        provider: 'archiveorg' as const,
        title: 'Metek Full Mix Fonction Radicale',
        artists: ['Metek'],
        crews: [],
        durationMs: 5_400_000,
        recordedAt: '2001-01-01',
        description: 'Archive description',
        genres: ['Freetekno'],
        artwork: ['https://archive.org/download/example/cover.jpg'],
        source: { provider: 'archiveorg' as const, url: 'https://archive.org/details/example', externalId: 'example' },
        externalIds: { archiveorg: 'example' },
        confidence: 0.82,
        reasons: ['artist token match', 'duration within 8%'],
        raw: {},
      }];
    },
  };

  const registry = {
    discovery: new Map([['archiveorg', archive]]),
    discogs: {
      async health() {
        return { id: 'discogs' as const, state: 'limited' as const, detail: 'public', checkedAt: new Date().toISOString() };
      },
      async enrichEntity(name: string, kind: 'artist' | 'crew' | 'label') {
        return [{
          kind,
          name,
          provider: 'discogs' as const,
          externalId: '123',
          url: 'https://www.discogs.com/artist/123',
          imageUrl: 'https://i.discogs.com/example.jpg',
        }];
      },
    },
  } as unknown as ProviderRegistry;

  const result = await enrichMix(registry, {
    title: 'Full Mix- Fonction radicale',
    artists: ['Metek'],
    durationMs: 0,
    genres: [],
    artwork: [],
    sources: [{ provider: 'freeteknomusic', url: 'https://archive.freeteknomusic.org/metek/' }],
  });

  assert.equal(result.patch.durationMs, 5_400_000);
  assert.equal(result.patch.description, 'Archive description');
  assert.deepEqual(result.patch.genres, ['Freetekno']);
  assert.equal(result.patch.artwork?.[0]?.provider, 'archiveorg');
  assert.equal(result.patch.externalIds?.archiveorg, 'example');
  assert.equal(result.patch.externalIds?.discogsArtist, '123');
  assert.ok(result.patch.sources?.some((source) => source.provider === 'archiveorg'));
  assert.ok(result.patch.sources?.some((source) => source.provider === 'discogs'));
  assert.equal(result.candidates.length, 1);
});

test('enrichMix skips an unavailable SoundCloud provider before search', async () => {
  let searched = false;
  const soundcloud = {
    id: 'soundcloud' as const,
    async health() {
      return { id: 'soundcloud' as const, state: 'limited' as const, detail: 'token missing', checkedAt: new Date().toISOString() };
    },
    async search() {
      searched = true;
      return [];
    },
  };

  const registry = {
    discovery: new Map([['soundcloud', soundcloud]]),
    discogs: {
      async health() {
        return { id: 'discogs' as const, state: 'offline' as const, detail: 'unavailable', checkedAt: new Date().toISOString() };
      },
      async enrichEntity() {
        return [];
      },
    },
  } as unknown as ProviderRegistry;

  const result = await enrichMix(registry, {
    title: 'Unknown warehouse set',
    artists: ['Artist'],
    sources: [],
  });

  assert.equal(searched, false);
  assert.ok(result.failures.some((failure) => failure.provider === 'soundcloud'));
});

test('enrichMix queries only explicitly selected providers', async () => {
  let archiveSearched = false;
  let soundcloudSearched = false;
  let discogsChecked = false;
  const makeProvider = (id: 'archiveorg' | 'soundcloud', mark: () => void) => ({
    id,
    async health() { return { id, state: 'ready' as const, detail: 'ok', checkedAt: new Date().toISOString() }; },
    async search() { mark(); return []; },
  });
  const registry = {
    discovery: new Map([
      ['archiveorg', makeProvider('archiveorg', () => { archiveSearched = true; })],
      ['soundcloud', makeProvider('soundcloud', () => { soundcloudSearched = true; })],
    ]),
    discogs: {
      async health() { discogsChecked = true; return { state: 'ready' as const, detail: 'ok' }; },
      async enrichEntity() { return []; },
    },
  } as unknown as ProviderRegistry;

  const result = await enrichMix(registry, {
    title: 'Known mix', artists: ['Known artist'], providers: ['archiveorg'],
  });

  assert.equal(archiveSearched, true);
  assert.equal(soundcloudSearched, false);
  assert.equal(discogsChecked, false);
  assert.deepEqual(result.attempted, ['archiveorg']);
});

test('enrichMix refreshes provider capabilities even when that provider already has a linked source', async () => {
  let searched = false;
  const soundcloud = {
    id: 'soundcloud' as const,
    async health() {
      return { id: 'soundcloud' as const, state: 'ready' as const, detail: 'ok', checkedAt: new Date().toISOString() };
    },
    async search() {
      searched = true;
      return [];
    }
  };
  const registry = {
    discovery: new Map([['soundcloud', soundcloud]]),
    discogs: { async health() { return { id: 'discogs' as const, state: 'offline' as const, detail: 'unavailable', checkedAt: new Date().toISOString() }; } }
  } as unknown as ProviderRegistry;
  const result = await enrichMix(registry, {
    title: 'Known mix', artists: ['Known artist'],
    sources: [{ provider: 'soundcloud', url: 'https://soundcloud.com/artist/known-mix' }]
  });
  assert.equal(searched, true);
  assert.ok(result.attempted.includes('soundcloud'));
});

test('enrichMix fills cover from an indexed SoundCloud source without replacing canonical metadata', async () => {
  const registry = {
    discovery: new Map(),
    soundcloud: {
      async lookupArtwork() { return 'https://i1.sndcdn.com/artworks-test-t500x500.jpg'; },
    },
    discogs: {
      async health() { return { state: 'offline', detail: 'unavailable' }; },
    },
  } as unknown as ProviderRegistry;
  const result = await enrichMix(registry, {
    title: 'Canonical title',
    artists: ['Known artist'],
    description: 'Good canonical description',
    artwork: [],
    sources: [{ provider: 'soundcloud', url: 'https://soundcloud.com/artist/long-mix' }],
  });
  assert.equal(result.patch.artwork?.[0]?.provider, 'soundcloud');
  assert.equal(result.patch.artwork?.[0]?.kind, 'cover');
  assert.equal(result.patch.description, undefined);
  assert.deepEqual(result.attempted, ['soundcloud', 'discogs']);
});

test('YouTube cover requires a strong title and duration match for a curator upload', async () => {
  const youtube = {
    id: 'youtube' as const,
    async health() { return { id: 'youtube' as const, state: 'ready' as const, detail: 'ok', checkedAt: new Date().toISOString() }; },
    async search() { return [
      { provider: 'youtube' as const, title: 'Live Mackitek Koalisson III', artists: ['Archive Curator'], crews: [], durationMs: 2_650_000,
        artwork: ['https://i.ytimg.com/vi/vi5miMVpmuI/hqdefault.jpg'], source: { provider: 'youtube' as const, url: 'https://www.youtube.com/watch?v=vi5miMVpmuI' }, confidence: 0.8, reasons: [], raw: {} },
      { provider: 'youtube' as const, title: 'Another Mackitek Koalisson Show', artists: ['Archive Curator'], crews: [], durationMs: 5_000_000,
        artwork: ['https://i.ytimg.com/vi/unrelated00/hqdefault.jpg'], source: { provider: 'youtube' as const, url: 'https://www.youtube.com/watch?v=unrelated00' }, confidence: 0.79, reasons: [], raw: {} },
    ]; },
  };
  const registry = { discovery: new Map([['youtube', youtube]]), discogs: { async health() { return { state: 'offline', detail: 'unavailable' }; } } } as unknown as ProviderRegistry;
  const result = await enrichMix(registry, { title: 'Kan10 Live Mackitek Koalisson III', artists: ['Mackitek'], durationMs: 2_641_424, artwork: [], sources: [] });
  assert.equal(result.patch.artwork?.[0]?.url, 'https://i.ytimg.com/vi/vi5miMVpmuI/hqdefault.jpg');
  assert.ok(!result.patch.sources?.some((source) => source.url.includes('unrelated00')));
});
