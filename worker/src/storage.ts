import { createHash, createHmac, randomUUID } from 'node:crypto';
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
export interface S3SignInput {
  method: 'GET' | 'PUT';
  url: URL;
  region: string;
  accessKey: string;
  secretKey: string;
  payload: Buffer;
  /** ISO basic timestamp; injectable for deterministic tests. */
  amzDate?: string;
}

function signKey(key: string | Buffer, data: string): Buffer {
  return createHmac('sha256', key).update(data, 'utf8').digest();
}

/**
 * AWS Signature Version 4 for a single S3 object request. Kept dependency-free so
 * the worker has no SDK in its image; the canonical request and key derivation
 * follow the documented S3 algorithm.
 */
export function signAwsV4(input: S3SignInput): { url: string; headers: Record<string, string> } {
  const amzDate = input.amzDate ?? new Date().toISOString().replace(/[:-]|\.\d{3}/g, '');
  const dateStamp = amzDate.slice(0, 8);
  const payloadHash = sha256(input.payload);
  const canonicalHeaders = `host:${input.url.host}\nx-amz-content-sha256:${payloadHash}\nx-amz-date:${amzDate}\n`;
  const signedHeaders = 'host;x-amz-content-sha256;x-amz-date';
  const canonicalRequest = [input.method, input.url.pathname, '', canonicalHeaders, signedHeaders, payloadHash].join('\n');
  const scope = `${dateStamp}/${input.region}/s3/aws4_request`;
  const stringToSign = ['AWS4-HMAC-SHA256', amzDate, scope, createHash('sha256').update(canonicalRequest, 'utf8').digest('hex')].join('\n');
  const kDate = signKey(`AWS4${input.secretKey}`, dateStamp);
  const kRegion = signKey(kDate, input.region);
  const kService = signKey(kRegion, 's3');
  const kSigning = signKey(kService, 'aws4_request');
  const signature = signKey(kSigning, stringToSign).toString('hex');
  return {
    url: input.url.toString(),
    headers: {
      'x-amz-content-sha256': payloadHash,
      'x-amz-date': amzDate,
      authorization: `AWS4-HMAC-SHA256 Credential=${input.accessKey}/${scope}, SignedHeaders=${signedHeaders}, Signature=${signature}`,
    },
  };
}

export class S3ObjectStore implements ObjectStore {
  constructor(
    private endpoint: string,
    private bucket: string,
    private accessKey: string,
    private secretKey: string,
    private region = 'us-east-1',
  ) {}

  private signedRequest(method: 'GET' | 'PUT', key: string, payload: Buffer = Buffer.alloc(0)): { url: string; headers: Record<string, string> } {
    assertSafeKey(key);
    const url = new URL(`${this.endpoint.replace(/\/$/, '')}/${this.bucket}/${key}`);
    return signAwsV4({ method, url, region: this.region, accessKey: this.accessKey, secretKey: this.secretKey, payload });
  }

  async putPrivate(key: string, bytes: Buffer, expectedSha256: string): Promise<void> {
    const actual = sha256(bytes);
    if (actual !== expectedSha256) throw new Error(`Hash mismatch: expected ${expectedSha256}, got ${actual}`);
    const request = this.signedRequest('PUT', key, bytes);
    const response = await fetch(request.url, { method: 'PUT', headers: request.headers, body: new Uint8Array(bytes) });
    if (!response.ok) throw new Error(`S3 upload failed with ${response.status}`);
  }

  async get(key: string): Promise<Buffer> {
    const request = this.signedRequest('GET', key);
    const response = await fetch(request.url, { headers: request.headers });
    if (!response.ok) throw new Error(`S3 download failed with ${response.status}`);
    return Buffer.from(await response.arrayBuffer());
  }
}

export function createStore(config: WorkerConfig): ObjectStore {
  if (config.storage.driver === 's3') {
    return new S3ObjectStore(
      config.storage.s3Endpoint!,
      config.storage.s3Bucket!,
      config.storage.s3AccessKey!,
      config.storage.s3SecretKey!,
      config.storage.s3Region ?? 'us-east-1',
    );
  }
  return new LocalObjectStore(config.storage.localRoot);
}
