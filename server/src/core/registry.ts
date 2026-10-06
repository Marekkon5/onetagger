import type { DiscoveryProvider, ProviderHealth, SearchQuery } from '../domain.js';
import { ArchiveOrgProvider } from '../providers/archiveorg.js';
import { DiscogsEnricher } from '../providers/discogs.js';
import { FreeteknomusicProvider } from '../providers/freeteknomusic.js';
import { SoundCloudProvider } from '../providers/soundcloud.js';
import { YouTubeProvider } from '../providers/youtube.js';
import { HearthisProvider } from '../providers/hearthis.js';

export class ProviderRegistry {
  readonly discovery = new Map<string, DiscoveryProvider>();
  readonly discogs = new DiscogsEnricher();
  readonly soundcloud = new SoundCloudProvider();
  readonly youtube = new YouTubeProvider();
  readonly hearthis = new HearthisProvider();

  constructor() {
    for (const provider of [new FreeteknomusicProvider(), this.soundcloud, new ArchiveOrgProvider(), this.youtube, this.hearthis]) {
      this.discovery.set(provider.id, provider);
    }
  }

  async health(): Promise<ProviderHealth[]> {
    const results = await Promise.allSettled([
      ...[...this.discovery.values()].map((provider) => provider.health()),
      this.discogs.health(),
    ]);
    return results.map((result, index) => result.status === 'fulfilled'
      ? result.value
      : { id: (['freeteknomusic', 'soundcloud', 'archiveorg', 'youtube', 'hearthis', 'discogs'] as const)[index], state: 'offline', detail: String(result.reason), checkedAt: new Date().toISOString() });
  }

  search(providerId: string, query: SearchQuery, signal?: AbortSignal) {
    const provider = this.discovery.get(providerId);
    if (!provider) throw new Error(`Unknown discovery provider: ${providerId}`);
    return provider.search(query, signal);
  }
}
