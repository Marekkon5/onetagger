import {
  ImportMetadataFactsSchema,
  type ImportJob,
  type ImportMetadataFacts,
  type ProviderId,
} from '@syco23/catalog-domain';
import { createImportStore, sha256, type ObjectStore } from './storage.js';

export interface ImportArtifactView {
  role: string;
  objectKey: string;
  sha256: string;
  sizeBytes?: number | null;
}

/** Metadata snapshots are bounded: provider facts are small JSON documents. */
export const MAX_METADATA_ARTIFACT_BYTES = 256 * 1024;

export class ImportFinalizeError extends Error {
  constructor(message: string, readonly statusCode: number) {
    super(message);
  }
}

/**
 * Read and validate the worker's metadata snapshot. Finalize must build the
 * canonical record from these sourced facts; a missing or mismatched snapshot
 * is a hard error rather than a placeholder record.
 */
export async function readMetadataFacts(
  job: ImportJob,
  artifacts: ImportArtifactView[],
  store: ObjectStore = createImportStore(),
): Promise<ImportMetadataFacts | undefined> {
  const artifact = artifacts.find((entry) => entry.role === 'metadata');
  if (!artifact) return undefined;
  if (artifact.sizeBytes !== null && artifact.sizeBytes !== undefined && artifact.sizeBytes > MAX_METADATA_ARTIFACT_BYTES) {
    throw new ImportFinalizeError('Metadata snapshot exceeds the allowed size', 409);
  }
  const bytes = await store.get(artifact.objectKey);
  if (bytes.length > MAX_METADATA_ARTIFACT_BYTES) {
    throw new ImportFinalizeError('Metadata snapshot exceeds the allowed size', 409);
  }
  if (sha256(bytes) !== artifact.sha256) {
    throw new ImportFinalizeError('Metadata snapshot does not match its registered hash', 409);
  }
  let raw: unknown;
  try {
    raw = JSON.parse(bytes.toString('utf8'));
  } catch {
    throw new ImportFinalizeError('Metadata snapshot is not valid JSON', 409);
  }
  try {
    return ImportMetadataFactsSchema.parse(raw);
  } catch {
    throw new ImportFinalizeError('Metadata snapshot does not match the import facts contract', 409);
  }
}

export function providerResourceType(provider: ProviderId): string {
  if (provider === 'youtube') return 'video';
  if (provider === 'soundcloud' || provider === 'hearthis') return 'track';
  if (provider === 'mixcloud') return 'cloudcast';
  if (provider === 'archiveorg') return 'item';
  if (provider === 'discogs') return 'release';
  if (provider === 'musicbrainz') return 'recording';
  return 'recording';
}
