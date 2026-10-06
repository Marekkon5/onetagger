import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, normalize, sep } from 'node:path';
import type { WorkerConfig } from './config.js';

export type ArtifactRole = 'original' | 'staging' | 'normalized' | 'artwork' | 'metadata';

/**
 * Object keys are derived from job id, role and content hash only. The original
 * filename, title or uploader name never becomes part of a storage path.
 */
export function objectKey(jobId: string, role: ArtifactRole, sha256: string, extension: string): string {
  if (!/^imp_[a-f0-9]+$/.test(jobId)) throw new Error('Job id is not a valid import id');
  if (!/^[0-9a-f]{64}$/.test(sha256)) throw new Error('sha256 must be a lowercase hex digest');
  // Tight allowlist: codecs/containers the pipeline can actually produce.
  const ext = extension.replace(/^\./, '').toLowerCase().replace(/[^a-z0-9]/g, '');
  if (!['mp3', 'json', 'jpg', 'jpeg', 'png', 'wav', 'flac', 'm4a', 'opus', 'ogg'].includes(ext)) {
    throw new Error('Unsupported artifact extension');
  }
  return `imports/${jobId}/${role}/${sha256}.${ext}`;
}

export function sha256(bytes: Buffer): string {
  return createHash('sha256').update(bytes).digest('hex');
}

export interface ObjectStore {
  putPrivate(key: string, bytes: Buffer, expectedSha256: string): Promise<void>;
  get(key: string): Promise<Buffer>;
}

function assertSafeKey(key: string): void {
  if (!/^imports\/imp_[a-f0-9]+\/(original|staging|normalized|artwork|metadata)\/[0-9a-f]{64}\.[a-z0-9]+$/.test(key)) {
    throw new Error('Object key does not match the expected shape');
  }
  const normalized = normalize(key);
  if (normalized.includes('..') || normalized.startsWith(sep)) {
    throw new Error('Object key contains unsafe segments');
  }
}

export class LocalObjectStore implements ObjectStore {
  constructor(private root: string) {}

  private resolve(key: string): string {
    assertSafeKey(key);
    const full = join(this.root, key);
    if (!normalize(full).startsWith(normalize(this.root) + sep)) throw new Error('Object key escapes the storage root');
    return full;
  }

  async putPrivate(key: string, bytes: Buffer, expectedSha256: string): Promise<void> {
    const actual = sha256(bytes);
    if (actual !== expectedSha256) throw new Error(`Hash mismatch: expected ${expectedSha256}, got ${actual}`);
    const target = this.resolve(key);
    await mkdir(dirname(target), { recursive: true });
    // Write to a temp sibling then rename: readers never observe partial objects.
    const temp = `${target}.${randomUUID()}.part`;
    await writeFile(temp, bytes, { mode: 0o600 });
    const { rename } = await import('node:fs/promises');
    await rename(temp, target);
  }

  async get(key: string): Promise<Buffer> {
    return readFile(this.resolve(key));
  }
}

/** Minimal S3-compatible store (PUT/GET object) for private buckets. */
export class S3ObjectStore implements ObjectStore {
  constructor(private endpoint: string, private bucket: string, private accessKey: string, private secretKey: string) {}

  async putPrivate(key: string, bytes: Buffer, expectedSha256: string): Promise<void> {
    const actual = sha256(bytes);
    if (actual !== expectedSha256) throw new Error(`Hash mismatch: expected ${expectedSha256}, got ${actual}`);
    assertSafeKey(key);
    const url = `${this.endpoint.replace(/\/$/, '')}/${this.bucket}/${key}`;
    const response = await fetch(url, { method: 'PUT', body: new Uint8Array(bytes) });
    if (!response.ok) throw new Error(`S3 upload failed with ${response.status}`);
  }

  async get(key: string): Promise<Buffer> {
    assertSafeKey(key);
    const url = `${this.endpoint.replace(/\/$/, '')}/${this.bucket}/${key}`;
    const response = await fetch(url);
    if (!response.ok) throw new Error(`S3 download failed with ${response.status}`);
    return Buffer.from(await response.arrayBuffer());
  }
}

export function createStore(config: WorkerConfig): ObjectStore {
  if (config.storage.driver === 's3') {
    return new S3ObjectStore(config.storage.s3Endpoint!, config.storage.s3Bucket!, config.storage.s3AccessKey!, config.storage.s3SecretKey!);
  }
  return new LocalObjectStore(config.storage.localRoot);
}
