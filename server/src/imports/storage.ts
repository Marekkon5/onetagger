import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { dirname, join, normalize, sep } from 'node:path';

export type ArtifactRole = 'original' | 'staging' | 'normalized' | 'artwork' | 'metadata';

const OBJECT_KEY = /^imports\/imp_[a-f0-9]+\/(original|staging|normalized|artwork|metadata)\/[0-9a-f]{64}\.[a-z0-9]+$/;

export function objectKey(jobId: string, role: ArtifactRole, digest: string, extension: string): string {
  if (!/^imp_[a-f0-9]+$/.test(jobId)) throw new Error('Job id is not a valid import id');
  if (!/^[0-9a-f]{64}$/.test(digest)) throw new Error('sha256 must be a lowercase hex digest');
  const ext = extension.replace(/^\./, '').toLowerCase().replace(/[^a-z0-9]/g, '');
  if (!['mp3', 'json', 'jpg', 'jpeg', 'png', 'wav', 'flac', 'm4a', 'opus', 'ogg'].includes(ext)) {
    throw new Error('Unsupported artifact extension');
  }
  return `imports/${jobId}/${role}/${digest}.${ext}`;
}

export function sha256(bytes: Buffer): string {
  return createHash('sha256').update(bytes).digest('hex');
}

export function extensionFromMime(mimeType: string | undefined): string | undefined {
  const mime = (mimeType ?? '').split(';')[0].trim().toLowerCase();
  if (mime === 'audio/mpeg' || mime === 'audio/mp3') return 'mp3';
  if (mime === 'audio/wav' || mime === 'audio/x-wav' || mime === 'audio/wave') return 'wav';
  if (mime === 'audio/flac') return 'flac';
  if (mime === 'audio/mp4' || mime === 'audio/x-m4a' || mime === 'audio/aac') return 'm4a';
  if (mime === 'audio/ogg' || mime === 'application/ogg') return 'ogg';
  if (mime === 'audio/opus') return 'opus';
  return undefined;
}

function assertSafeKey(key: string): void {
  if (!OBJECT_KEY.test(key)) throw new Error('Object key does not match the expected shape');
  const normalized = normalize(key);
  if (normalized.includes('..') || normalized.startsWith(sep)) {
    throw new Error('Object key contains unsafe segments');
  }
}

export interface ObjectStore {
  putPrivate(key: string, bytes: Buffer, expectedSha256: string): Promise<void>;
  get(key: string): Promise<Buffer>;
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
    const temp = `${target}.${randomUUID()}.part`;
    await writeFile(temp, bytes, { mode: 0o600 });
    await rename(temp, target);
  }

  async get(key: string): Promise<Buffer> {
    return readFile(this.resolve(key));
  }
}

export class S3ObjectStore implements ObjectStore {
  constructor(private endpoint: string, private bucket: string) {}

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

export function createImportStore(): ObjectStore {
  const driver = process.env.IMPORT_STORAGE_DRIVER || 'local';
  if (driver === 's3') {
    const endpoint = process.env.IMPORT_S3_ENDPOINT;
    const bucket = process.env.IMPORT_S3_BUCKET;
    if (!endpoint || !bucket) throw new Error('IMPORT_S3_ENDPOINT and IMPORT_S3_BUCKET are required');
    return new S3ObjectStore(endpoint, bucket);
  }
  return new LocalObjectStore(process.env.IMPORT_STORAGE_LOCAL_ROOT || './data/imports');
}
