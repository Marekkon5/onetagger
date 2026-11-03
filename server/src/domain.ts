export type ProviderId = 'freeteknomusic' | 'soundcloud' | 'archiveorg' | 'discogs' | 'youtube' | 'hearthis' | 'mixcloud';

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
  aliases?: string[];
  realName?: string;
  websiteUrls?: string[];
  aliasRefs?: EntityRef[];
  groups?: EntityRef[];
  members?: EntityRef[];
  parent?: EntityRef;
  subLabels?: EntityRef[];
}

export interface MixCandidate {
  provider: ProviderId;
  title: string;
  artists: string[];
  crews: string[];
  durationMs?: number;
  recordedAt?: string;
  uploader?: string;
  uploadedAt?: string;
  fieldEvidence?: Record<string, 'direct' | 'parsed' | 'analysis'>;
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
