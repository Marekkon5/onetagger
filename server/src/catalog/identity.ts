import { randomUUID } from 'node:crypto';
import {
  ImportCandidateSchema,
  ImportResultSchema,
  RecordingDateSchema,
  type ImportCandidate,
  type ImportResult,
  type ProviderRef
} from '@syco23/catalog-domain';
import { claimsFromProvider } from './provider-claims.js';
import { applyClaims } from './merge.js';
import type { CatalogRepository } from './repository.js';
import type { CuratorActor } from '../auth/curator.js';

export function importCandidate(repository: CatalogRepository, input: ImportCandidate, actor: CuratorActor): ImportResult {
  void actor;
  const candidate = ImportCandidateSchema.parse(input);
  const ref: ProviderRef = candidate.source;
  const existingId = repository.findBySource(ref) ?? repository.findByProvider(ref);
  if (existingId) {
    const existing = repository.getRecord(existingId);
    if (!existing) throw new Error(`Provider identity points to missing record ${existingId}`);
    return ImportResultSchema.parse({ record: existing, outcome: 'existing' });
  }

  const now = new Date().toISOString();
  const recordingDate = candidate.recordedAt
    ? RecordingDateSchema.safeParse(/^(\d{4}|\d{4}-\d{2}|\d{4}-\d{2}-\d{2})$/.test(candidate.recordedAt)
      ? { value: candidate.recordedAt, precision: candidate.recordedAt.length === 4 ? 'year' : candidate.recordedAt.length === 7 ? 'month' : 'day' }
      : undefined).data
    : undefined;
  const id = `mix_${randomUUID()}`;
  const record = {
    kind: 'mix' as const,
    id,
    createdAt: now,
    updatedAt: now,
    revision: 1,
    verification: 'curator-confirmed' as const,
    reviewState: 'ready' as const,
    title: candidate.title,
    ...(recordingDate ? { recordingDate } : {}),
    people: [],
    eventIds: [],
    genres: [],
    styles: [],
    assets: [],
    sources: [{ ...ref, addedAt: now }]
  };

  repository.transaction((tx) => {
    tx.saveRecord(record);
    tx.addProviderSource(record.id, ref);
  });

  const metadata = {
    provider: ref,
    sourceUrl: ref.url!,
    observedAt: now,
    match: { status: 'confirmed' as const, explanation: 'The curator selected this provider resource for the shared archive' },
    facts: {
      title: candidate.title,
      ...(candidate.durationMs ? { durationMs: candidate.durationMs } : {}),
      ...(candidate.description ? { description: candidate.description } : {}),
      ...(candidate.genres.length ? { genres: candidate.genres } : {})
    }
  };
  applyClaims(repository, claimsFromProvider(record, metadata));

  if (candidate.artwork.length) {
    const artworkMetadata = {
      ...metadata,
      match: { status: 'possible' as const, explanation: 'Provider artwork requires cover-role confirmation before selection' },
      facts: { cover: { role: 'mix-cover', url: candidate.artwork[0] } }
    };
    applyClaims(repository, claimsFromProvider(repository.getRecord(record.id)!, artworkMetadata));
  }

  const imported = repository.getRecord(record.id);
  if (!imported) throw new Error('Imported record could not be reloaded');
  return ImportResultSchema.parse({ record: imported, outcome: 'created' });
}