import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { LocalObjectStore, objectKey, sha256 } from './storage.js';

const jobId = 'imp_' + 'a'.repeat(24);
const digest = 'b'.repeat(64);

test('object keys derive from job, role and hash only', () => {
  assert.equal(objectKey(jobId, 'normalized', digest, 'mp3'), `imports/${jobId}/normalized/${digest}.mp3`);
  assert.equal(objectKey(jobId, 'metadata', digest, 'json'), `imports/${jobId}/metadata/${digest}.json`);
  assert.throws(() => objectKey('evil_../x', 'normalized', digest, 'mp3'), /valid import id/);
  assert.throws(() => objectKey(jobId, 'normalized', 'short', 'mp3'), /hex/);
  assert.throws(() => objectKey(jobId, 'normalized', digest, 'php'), /extension/);
  assert.throws(() => objectKey(jobId, 'normalized', digest, '../../x'), /extension/);
});

test('local store writes atomically and verifies hashes', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-store-'));
  try {
    const store = new LocalObjectStore(directory);
    const bytes = Buffer.from('mix-audio Fixture');
    const hash = sha256(bytes);
    const key = objectKey(jobId, 'normalized', hash, 'mp3');
    await store.putPrivate(key, bytes, hash);
    assert.deepEqual(await store.get(key), bytes);
    await assert.rejects(() => store.putPrivate(key, bytes, digest), /Hash mismatch/);
    await assert.rejects(() => store.get('imports/evil/key.mp3'), /expected shape/);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
