export type ProviderId = 'freeteknomusic' | 'soundcloud' | 'archiveorg' | 'discogs' | 'youtube' | 'hearthis';

export interface SourceRef {
  provider: ProviderId;
  url: string;
  externalId?: string;
}

export interface EntityRef {
  kind: 'artist' | 'crew' | 'label';
  name: string;
  provider?: ProviderId;
  externalId?: string;
  url?: string;
  imageUrl?: string;
  profile?: string;
}

export interface MixCandidate {
  provider: ProviderId;
  title: string;
  artists: string[];
  crews: string[];
  durationMs?: number;
  recordedAt?: string;
  description?: string;
  genres?: string[];
  artwork?: string[];
  source: SourceRef;
  externalIds?: Record<string, string>;
  entities?: EntityRef[];
  confidence: number;
  reasons: string[];
  raw: Record<string, unknown>;
}

export interface SearchQuery {
  q?: string;
  url?: string;
  artist?: string;
  crew?: string;
  minDurationMs?: number;
  durationExpectedMs?: number;
  maxDepth?: number;
  maxItems?: number;
  limit?: number;
}

export interface ProviderHealth {
  id: ProviderId;
  state: 'ready' | 'limited' | 'offline';
  detail: string;
  checkedAt: string;
}

export interface DiscoveryProvider {
  id: ProviderId;
  search(query: SearchQuery, signal?: AbortSignal): Promise<MixCandidate[]>;
  health(signal?: AbortSignal): Promise<ProviderHealth>;
}

export type {
  CatalogDetail,
  CatalogRecord,
  EntityRecord,
  EntityRole as CatalogEntityRole,
  EventRecord,
  FieldClaim,
  IndexKind,
  LegacyMix,
  MixRecord,
  ProviderRef,
  RecordId,
  ReviewItem
} from '@syco23/catalog-domain';
