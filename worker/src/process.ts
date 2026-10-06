import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { durationComponent, scoreEvidence } from '@syco23/catalog-domain';
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
    });

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
      });
      raw = fetched.bytes;
      finalUrl = fetched.finalUrl;
    } else {
      throw new RetryableError('No downloadable audio URL was resolved');
    }
    const rawSha = sha256(raw);
    const rawPath = join(workdir, 'input.raw');
    await writeFile(rawPath, raw, { mode: 0o600 });

    await client.event(job.id, 'probing', { state: 'probing' });
    const probe = await boundedFFprobe(rawPath, limits);
    if (!heartbeat.alive) throw new RetryableError('Lease lost during probe');

    await client.event(job.id, 'converting', { state: 'converting', durationMs: probe.durationMs });
    const outPath = join(workdir, 'normalized.mp3');
    await boundedFFmpegConvert({
      inputPath: rawPath,
      outputPath: outPath,
      bitrateKbps: 320,
      maxOutputBytes: limits.maxOutputBytes,
      timeoutMs: conversionTimeoutMs(probe.durationMs),
      metadata: { title: source.title ?? job.id, artist: job.provider },
    });
    if (!heartbeat.alive) throw new RetryableError('Lease lost during conversion');
    const { readFile, stat } = await import('node:fs/promises');
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
      I: 1, T: source.title ? 1 : 0.4,
      D: durationComponent(probe.durationMs, verified.durationMs),
      R: 0.2, P: 1, C: 0, X: 0,
      identityGate: true, rightsGate: true, noDirectConflict: true, providerPolicyAllowsUse: true,
    });
    await client.submitEvidence(job.id, [{ ...scored, field: 'audio' }]);
    await client.event(job.id, 'acquisition', { finalUrl, rawSha, normalizedSha });
  } finally {
    heartbeat.stop();
    await rm(workdir, { recursive: true, force: true });
  }
}

/** Metadata-only jobs: resolve, snapshot as private artifact, score, finish. */
export async function processMetadataJob(client: ControlClient, config: WorkerConfig, spec: WorkSpec): Promise<void> {
  const { job } = spec;
  const store = createStore(config);
  await client.event(job.id, 'resolving', { state: 'resolving' });
  const source = await resolveSource({ provider: job.provider, sourceUrl: job.sourceUrl, mode: 'metadata' });
  const snapshot = Buffer.from(JSON.stringify({
    provider: job.provider, sourceUrl: job.sourceUrl, title: source.title,
    artworkUrl: source.artworkUrl, observedAt: new Date().toISOString(),
  }), 'utf8');
  const snapshotSha = sha256(snapshot);
  const key = objectKey(job.id, 'metadata', snapshotSha, 'json');
  await store.putPrivate(key, snapshot, snapshotSha);
  await client.registerArtifact(job.id, {
    id: `art_${snapshotSha.slice(0, 24)}`, role: 'metadata', objectKey: key,
    sha256: snapshotSha, mimeType: 'application/json', sizeBytes: snapshot.length,
  });
  const scored = scoreEvidence({
    I: 1, T: source.title ? 0.8 : 0.2, D: 0, R: 0.3, P: 1, C: 0, X: 0,
    identityGate: true, rightsGate: true, noDirectConflict: true, providerPolicyAllowsUse: true,
  });
  await client.submitEvidence(job.id, [{ ...scored, field: 'metadata' }]);
}
