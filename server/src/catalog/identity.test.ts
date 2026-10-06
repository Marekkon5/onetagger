import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import type { ImportCandidate } from '@syco23/catalog-domain';
import { openCatalog } from './repository.js';
import { importCandidate } from './identity.js';

function candidate(provider: ImportCandidate['provider'], externalId: string): ImportCandidate {
  return {
    provider,
    title: 'Live Mackitek Koalisson III',
    artists: ['Archive Channel'],
    crews: [],
    durationMs: 2_641_424,
    description: 'Long-form recording',
    genres: ['Tekno'],
    artwork: ['https://i.ytimg.com/vi/vi5miMVpmuI/hqdefault.jpg'],
    source: { provider, resourceType: provider === 'youtube' ? 'video' : 'track', externalId, url: provider === 'youtube' ? `https://www.youtube.com/watch?v=${externalId}` : `https://soundcloud.com/mix/${externalId}` },
    confidence: 0.85,
    reasons: ['Provider resource selected by curator']
  };
}

function withCatalog(run: (repo: ReturnType<typeof openCatalog>) => void) {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-import-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  try { run(repo); }
  finally { repo.close(); rmSync(directory, { recursive: true, force: true }); }
}

test('repeated source imports return one stable canonical record', () => {
  withCatalog((repo) => {
    const first = importCandidate(repo, candidate('youtube', 'video-1'), { sessionId: 'curator' });
    const second = importCandidate(repo, candidate('youtube', 'video-1'), { sessionId: 'curator' });
    assert.equal(first.outcome, 'created');
    assert.equal(second.outcome, 'existing');
    assert.equal(first.record.id, second.record.id);
    assert.equal(repo.listIndex('mix', { page: 1, pageSize: 50 }).total, 1);
  });
});

test('matching names across providers remain distinct and uploader names are not performers', () => {
  withCatalog((repo) => {
    const youtube = importCandidate(repo, candidate('youtube', 'same-id'), { sessionId: 'curator' });
    const soundcloud = importCandidate(repo, candidate('soundcloud', 'same-id'), { sessionId: 'curator' });
    assert.notEqual(youtube.record.id, soundcloud.record.id);
    assert.equal(youtube.record.kind === 'mix' ? youtube.record.people.length : -1, 0);
    assert.equal(youtube.record.kind === 'mix' ? youtube.record.assets.length : -1, 0);
    assert.deepEqual(repo.listReview().map((item) => item.field).sort(), ['cover', 'cover']);
  });
});