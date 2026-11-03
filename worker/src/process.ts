import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  ImportMetadataFactsSchema,
  RecordingDateSchema,
  durationComponent,
  scoreEvidence,
  type ImportMetadataFacts,
} from '@syco23/catalog-domain';
import { ControlClient, type WorkSpec } from './control.js';
import { boundedFFmpegConvert, boundedFFprobe, conversionTimeoutMs } from './ffmpeg.js';
import { NonRetryableError, resolveSource } from './resolvers.js';
import { boundedFetch } from './security.js';
import { createStore, objectKey, sha256 } from './storage.js';
import type { WorkerConfig } from './config.js';

export class RetryableError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'RetryableError';
  }
}

/**
 * Lease heartbeats keep the outbox lease alive during long FFmpeg runs.
 * Returns false when the lease was lost: the caller must stop immediately
 * because another worker may have claimed the job.
 */
function startHeartbeat(client: ControlClient, jobId: string, heartbeatMs: number) {
  let leaseAlive = true;
  const timer = setInterval(async () => {
    try {
      const ok = await client.heartbeat(jobId);
      if (!ok) { leaseAlive = false; clearInterval(timer); }
    } catch {
      // A failed heartbeat is not fatal by itself; loss becomes visible at
      // the next renewal. Do not clear leaseAlive on transient errors.
    }
  }, heartbeatMs);
  if (typeof timer.unref === 'function') timer.unref();
  return { stop: () => clearInterval(timer), get alive() { return leaseAlive; } };
}

/** Persist the sourced facts so finalize can build a real record for any job. */
async function writeMetadataSnapshot(
  store: ReturnType<typeof createStore>,
  client: ControlClient,
  jobId: string,
  facts: ImportMetadataFacts,
): Promise<void> {
  const snapshot = Buffer.from(JSON.stringify(facts), 'utf8');
  const snapshotSha = sha256(snapshot);
  const key = objectKey(jobId, 'metadata', snapshotSha, 'json');
  await store.putPrivate(key, snapshot, snapshotSha);
  await client.registerArtifact(jobId, {
    id: `art_${snapshotSha.slice(0, 24)}`, role: 'metadata', objectKey: key,
    sha256: snapshotSha, mimeType: 'application/json', sizeBytes: snapshot.length,
  });
}

export async function processAudioJob(client: ControlClient, config: WorkerConfig, spec: WorkSpec): Promise<void> {
  const { job, limits } = spec;
  const store = createStore(config);
  const heartbeat = startHeartbeat(client, job.id, config.heartbeatMs);
  const workdir = await mkdtemp(join(tmpdir(), `syco23-import-${job.id.slice(0, 12)}-`));
  try {
    await client.event(job.id, 'resolving', { state: 'resolving' });
    const source = await resolveSource({
      provider: job.provider,
      sourceUrl: job.sourceUrl,
      mode: 'audio',
      originalObjectKey: spec.originalObjectKey,
      credentials: config.providers,
    });
    // Audio jobs carry their own facts too. Persisting them keeps finalize
    // uniform: it never has to guess a provider title from the URL.
    if (source.facts) await writeMetadataSnapshot(store, client, job.id, source.facts);

    await client.event(job.id, 'acquiring', { state: 'acquiring' });
    let raw: Buffer;
    let finalUrl: string;
    if (source.objectKey) {
      raw = await store.get(source.objectKey);
      if (raw.length > limits.maxInputBytes) throw new NonRetryableError('Original object exceeds the input quota');
      finalUrl = job.sourceUrl;
    } else if (source.audioUrl) {
      const fetched = await boundedFetch(source.audioUrl, {
        maxBytes: limits.maxInputBytes,
        timeoutMs: 5 * 60_000,
        ...(source.audioHeaders ? { headers: source.audioHeaders } : {}),
      });
      raw = fetched.bytes;
      finalUrl = fetched.finalUrl;
    } else {
      throw new RetryableError('No downloadable audio URL was resolved');
    }
    const rawSha = sha256(raw);
    const rawPath = join(workdir, 'input.raw');
    await writeFile(rawPath, raw, { mode: 0o600 });
    // FFmpeg reads from disk; drop the in-memory copy so peak RSS stays
    // bounded by a single buffer instead of input + output.
    raw = Buffer.alloc(0);

    await client.event(job.id, 'probing', { state: 'probing' });
    const probe = await boundedFFprobe(rawPath, limits);
    if (!heartbeat.alive) throw new RetryableError('Lease lost during probe');

    await client.event(job.id, 'converting', { state: 'converting', durationMs: probe.durationMs });
    const outPath = join(workdir, 'normalized.mp3');
    const { readFile, stat } = await import('node:fs/promises');
    const conversion = await boundedFFmpegConvert({
      inputPath: rawPath,
      outputPath: outPath,
      bitrateKbps: 320,
      maxOutputBytes: limits.maxOutputBytes,
      timeoutMs: conversionTimeoutMs(probe.durationMs),
      source: probe,
      limits,
      metadata: {
        ...(source.facts?.title ? { title: source.facts.title } : {}),
        ...(source.facts?.artists?.length ? { artist: source.facts.artists.join(', ') } : {}),
        ...(source.facts?.recordedAt ? { date: source.facts.recordedAt.slice(0, 4) } : {}),
      },
    });
    if (!heartbeat.alive) throw new RetryableError('Lease lost during conversion');
    const normalized = await readFile(outPath);
    const stated = await stat(outPath);
    if (stated.size > limits.maxOutputBytes) throw new NonRetryableError('Converted output exceeds the output quota');

    // Read-back verification: the file must re-probe with matching duration.
    await client.event(job.id, 'tagging', { state: 'tagging' });
    const verified = await boundedFFprobe(outPath, limits);
    const drift = Math.abs(verified.durationMs - probe.durationMs) / Math.max(1, probe.durationMs);
    if (drift > 0.02 && Math.abs(verified.durationMs - probe.durationMs) > 2000) {
      throw new NonRetryableError('Read-back duration verification failed');
    }

    const normalizedSha = sha256(normalized);
    const key = objectKey(job.id, 'normalized', normalizedSha, 'mp3');
    await store.putPrivate(key, normalized, normalizedSha);
    const artifactId = `art_${normalizedSha.slice(0, 24)}`;
    await client.registerArtifact(job.id, {
      id: artifactId, role: 'normalized', objectKey: key, sha256: normalizedSha,
      mimeType: 'audio/mpeg', sizeBytes: stated.size, codec: verified.codec, durationMs: verified.durationMs,
    });

    await client.event(job.id, 'scoring', { state: 'scoring' });
    const scored = scoreEvidence({
      I: 1, T: source.facts?.title ? 1 : 0.4,
      D: durationComponent(probe.durationMs, verified.durationMs),
      R: 0.2, P: 1, C: 0, X: 0,
      identityGate: true, rightsGate: true, noDirectConflict: true, providerPolicyAllowsUse: true,
    });
    await client.submitEvidence(job.id, [{ ...scored, field: 'audio' }]);
    await client.event(job.id, 'acquisition', {
      finalUrl, rawSha, normalizedSha, conversionMode: conversion.mode, outputBitrateKbps: conversion.bitrateKbps,
    });
  } finally {
    heartbeat.stop();
    await rm(workdir, { recursive: true, force: true });
  }
}

function recordingDateFact(value: string | undefined): unknown {
  if (!value) return undefined;
  const precision = value.length === 4 ? 'year' : value.length === 7 ? 'month' : 'day';
  const parsed = RecordingDateSchema.safeParse({ value, precision });
  return parsed.success ? parsed.data : undefined;
}

/**
 * Per-field evidence with the sourced value attached. Finalize reads these
 * values, so a claim never has to be reconstructed from a URL. Provider APIs
 * produce direct facts; filename/folder-derived freeteknomusic identity is
 * marked parsed and stays in Review.
 */
export function metadataEvidence(provider: string, facts: ImportMetadataFacts): unknown[] {
  const direct = provider !== 'freeteknomusic';
  const entries: Array<[string, unknown]> = [
    ['title', facts.title],
    ['description', facts.description],
    ['durationMs', facts.durationMs],
    ['genres', facts.genres?.length ? facts.genres : undefined],
    ['styles', facts.styles?.length ? facts.styles : undefined],
    ['recordingDate', recordingDateFact(facts.recordedAt)],
    ['cover', facts.artworkUrl ? { role: 'mix-cover', url: facts.artworkUrl } : undefined],
  ];
  return entries
    .filter(([, value]) => value !== undefined)
    .map(([field, value]) => ({
      ...scoreEvidence({
        I: 1, T: direct ? 0.9 : 0.5, D: 1, R: direct ? 0.9 : 0.5,
        P: 1, C: direct ? 1 : 0.4, X: 0,
        identityGate: true, rightsGate: true, noDirectConflict: true, providerPolicyAllowsUse: true,
      }),
      field,
      value,
    }));
}

/** Metadata-only jobs: resolve, snapshot full facts, score per field, finish. */
export async function processMetadataJob(client: ControlClient, config: WorkerConfig, spec: WorkSpec): Promise<void> {
  const { job } = spec;
  const store = createStore(config);
  await client.event(job.id, 'resolving', { state: 'resolving' });
  const source = await resolveSource({
    provider: job.provider,
    sourceUrl: job.sourceUrl,
    mode: 'metadata',
    credentials: config.providers,
  });
  if (!source.facts) throw new NonRetryableError('Provider resolver returned no metadata facts');
  const facts = ImportMetadataFactsSchema.parse(source.facts);
  await writeMetadataSnapshot(store, client, job.id, facts);
  const evidence = metadataEvidence(job.provider, facts);
  if (evidence.length === 0) throw new NonRetryableError('Provider returned no scorable metadata facts');
  await client.submitEvidence(job.id, evidence);
}
