import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { openCatalog } from '../../server/src/catalog/repository.js';
import { applyClaims } from '../../server/src/catalog/merge.js';
import { hashCuratorPassword } from '../../server/src/auth/curator.js';
import { ProviderRegistry } from '../../server/src/core/registry.js';
import { ArchiveOrgProvider } from '../../server/src/providers/archiveorg.js';
import { DiscogsEnricher } from '../../server/src/providers/discogs.js';
import { FreeteknomusicProvider } from '../../server/src/providers/freeteknomusic.js';
import { HearthisProvider } from '../../server/src/providers/hearthis.js';
import { MixcloudProvider } from '../../server/src/providers/mixcloud.js';
import { SoundCloudProvider } from '../../server/src/providers/soundcloud.js';
import { YouTubeProvider } from '../../server/src/providers/youtube.js';
import type { CatalogRecord, ProviderRef } from '@syco23/catalog-domain';

// A deterministic external provider boundary; all catalog/auth/routes/SQLite code remains real.
const nowForProvider = () => new Date().toISOString();
ProviderRegistry.prototype.health = async () => ['freeteknomusic', 'archiveorg', 'soundcloud', 'discogs', 'youtube', 'hearthis', 'mixcloud'].map(id => ({ id: id as any, state: id === 'youtube' ? 'ready' : 'offline', detail: 'Disposable e2e provider fixture', checkedAt: nowForProvider() }));
ProviderRegistry.prototype.search = async function(providerId, query) {
  if (providerId !== 'youtube') return [];
  const id = query.url ? new URL(query.url).searchParams.get('v') || 'e2eImport01' : 'e2eImport01';
  return [{ provider: 'youtube', title: 'Imported Click Recording', artists: [], crews: [], durationMs: 5400000, description: 'Verified recording metadata from deterministic provider fixture', artwork: ['https://i.ytimg.com/vi/e2eImport01/hqdefault.jpg'], source: { provider: 'youtube', url: `https://www.youtube.com/watch?v=${id}`, externalId: id }, confidence: 1, reasons: ['Explicit source identity'], raw: {} }];
};
YouTubeProvider.prototype.lookupVideo = async sourceUrl => ({ id: new URL(sourceUrl).searchParams.get('v')!, title: 'Imported Click Recording', artist: 'Uploader is not a performer', artworkUrl: 'https://i.ytimg.com/vi/e2eImport01/hqdefault.jpg' });

// Enrichment (`enrichCatalogRecord` -> `enrichMix`) resolves records through the raw
// provider adapters (provider.health() / provider.search() / artwork lookups) instead of
// through the registry, so those prototypes must be stubbed too. Without this the
// click-through enrichment reaches the live Archive.org, hearthis.at and Discogs endpoints
// and stalls past the Playwright assertion timeout.
const offlineHealth = (id: string) => async () => ({ id, state: 'offline', detail: 'Disposable e2e provider fixture', checkedAt: nowForProvider() });
ArchiveOrgProvider.prototype.health = offlineHealth('archiveorg');
ArchiveOrgProvider.prototype.search = async () => [];
FreeteknomusicProvider.prototype.health = offlineHealth('freeteknomusic');
FreeteknomusicProvider.prototype.search = async () => [];
HearthisProvider.prototype.health = offlineHealth('hearthis');
HearthisProvider.prototype.search = async () => [];
HearthisProvider.prototype.lookupTrack = async () => { throw new Error('Disposable e2e provider fixture'); };
HearthisProvider.prototype.lookupArtwork = async () => undefined;
SoundCloudProvider.prototype.health = async () => ({ id: 'soundcloud', state: 'limited', detail: 'Disposable e2e provider fixture', checkedAt: nowForProvider() });
SoundCloudProvider.prototype.search = async () => [];
SoundCloudProvider.prototype.resolvePublicArtwork = async () => ({ sourceUrl: '', artworkUrl: undefined });
SoundCloudProvider.prototype.lookupArtwork = async () => undefined;
MixcloudProvider.prototype.health = offlineHealth('mixcloud');
MixcloudProvider.prototype.search = async () => [];
MixcloudProvider.prototype.lookupCloudcast = async () => { throw new Error('Disposable e2e provider fixture'); };
MixcloudProvider.prototype.lookupArtwork = async () => undefined;
DiscogsEnricher.prototype.health = offlineHealth('discogs');
DiscogsEnricher.prototype.enrichEntity = async () => [];
DiscogsEnricher.prototype.hydrateEntity = async () => undefined;
YouTubeProvider.prototype.health = async () => ({ id: 'youtube', state: 'ready', detail: 'Disposable e2e provider fixture', checkedAt: nowForProvider() });
YouTubeProvider.prototype.search = async function (query: { url?: string }) {
  if (!query.url) return [];
  const id = new URL(query.url).searchParams.get('v') || 'e2eImport01';
  return [{ provider: 'youtube', title: 'Imported Click Recording', artists: [], crews: [], durationMs: 5400000, description: 'Verified recording metadata from deterministic provider fixture', artwork: ['https://i.ytimg.com/vi/e2eImport01/hqdefault.jpg'], source: { provider: 'youtube', url: `https://www.youtube.com/watch?v=${id}`, externalId: id }, confidence: 1, reasons: ['Explicit source identity'], raw: {} }];
};
YouTubeProvider.prototype.lookupArtwork = async () => 'https://i.ytimg.com/vi/e2eImport01/hqdefault.jpg';
const directory = mkdtempSync(join(tmpdir(), 'syco23-click-e2e-'));
process.on('exit', () => rmSync(directory, { recursive: true, force: true }));
process.env.CATALOG_DB_PATH = join(directory, 'catalog.sqlite');
process.env.CATALOG_MEDIA_PATH = join(directory, 'media');
process.env.PORT = '18787';
process.env.CORS_ORIGIN = 'http://localhost:15173';
process.env.CURATOR_PASSWORD_HASH = hashCuratorPassword('local-e2e-curator-12345');
process.env.NODE_ENV = 'test';
for (const key of ['SOUNDCLOUD_ACCESS_TOKEN', 'SOUNDCLOUD_CLIENT_ID', 'SOUNDCLOUD_CLIENT_SECRET', 'DISCOGS_TOKEN', 'YOUTUBE_API_KEY']) delete process.env[key];

const repository = openCatalog(process.env.CATALOG_DB_PATH);
const now = new Date().toISOString();
const base = { createdAt: now, updatedAt: now, revision: 1, verification: 'curator-confirmed' as const, reviewState: 'ready' as const };
const artistRef: ProviderRef = { provider: 'discogs', resourceType: 'artist', externalId: 'fixture-artist', url: 'https://www.discogs.com/artist/fixture-artist' };
const records: CatalogRecord[] = [
  { ...base, kind: 'entity', id: 'artist_e2e_001', displayName: 'DJ Live', roles: ['artist'], aliases: ['DJ LIVE'], profile: 'Shared artist profile', country: 'DE', assets: [], providerRefs: [artistRef], artist: { realName: 'Fixture Performer' } },
  { ...base, kind: 'entity', id: 'crew_e2e_001', displayName: 'Click Test Crew', roles: ['crew'], aliases: [], assets: [], providerRefs: [], crew: { websiteUrls: ['https://example.org/crew'] } },
  { ...base, kind: 'entity', id: 'crew_e2e_002', displayName: 'Duplicate Click Crew', roles: ['crew'], aliases: ['Crew alternative name'], country: 'FR', profile: 'Alternative crew profile', assets: [], providerRefs: [] },
  { ...base, kind: 'entity', id: 'label_e2e_001', displayName: 'Click Test Label', roles: ['label'], aliases: [], assets: [], providerRefs: [], label: { websiteUrls: ['https://example.org/label'] } },
  { ...base, kind: 'event', id: 'event_e2e_001', name: 'Click Test Event', startDate: { value: '2025-06', precision: 'month' }, venue: 'Fixture venue', locality: 'Berlin', country: 'DE', assets: [], sourceUrls: ['https://example.org/event'], mixIds: [] },
  { ...base, kind: 'mix', id: 'mix_e2e_001', title: 'Shared Click Recording', durationMs: 7200000, recordingDate: { value: '2024', precision: 'year' }, people: [{ entityId: 'artist_e2e_001', role: 'artist' }, { entityId: 'crew_e2e_001', role: 'crew' }, { entityId: 'label_e2e_001', role: 'label' }], eventIds: ['event_e2e_001'], genres: ['Tekno'], styles: [], assets: [], sources: [{ provider: 'youtube', resourceType: 'video', externalId: 'e2eVideo001', url: 'https://www.youtube.com/watch?v=e2eVideo001' }] },
  { ...base, kind: 'mix', id: 'mix_e2e_002', title: 'Second Shared Recording', durationMs: 3600000, people: [{ entityId: 'artist_e2e_001', role: 'artist' }], eventIds: [], genres: [], styles: [], assets: [], sources: [{ provider: 'hearthis', resourceType: 'track', externalId: 'e2e-track-002', url: 'https://hearthis.at/fixture/e2e-track-002/' }] },
  { ...base, kind: 'mix', id: 'mix_e2e_event', title: 'Event Only Recording', people: [], eventIds: ['event_e2e_001'], genres: [], styles: [], assets: [], sources: [] }
];
repository.transaction(tx => {
  for (const record of records) tx.saveRecord(record);
  tx.addProviderSource('artist_e2e_001', artistRef);
  for (const record of records) if (record.kind === 'mix') for (const source of record.sources) tx.addProviderSource(record.id, source);
});
applyClaims(repository, [{ targetRecordId: 'artist_e2e_001', field: 'country', value: 'FR', provider: artistRef, sourceUrl: artistRef.url!, observedAt: now, evidence: 'direct', matchExplanation: 'Disposable provider conflict proving selected country protection' }]);
repository.close();
await import('../../server/src/index.js');
