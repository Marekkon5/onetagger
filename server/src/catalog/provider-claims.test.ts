import assert from 'node:assert/strict';
import test from 'node:test';
import type { MixRecord } from '@syco23/catalog-domain';
import { claimsFromProvider, type ProviderMetadata } from './provider-claims.js';

const target: MixRecord = {
  kind: 'mix', id: 'mix_01J9CATALOGUE00000000000030',
  createdAt: '2026-10-03T12:00:00.000Z', updatedAt: '2026-10-03T12:00:00.000Z',
  revision: 1, verification: 'source-confirmed', reviewState: 'ready', title: 'Known mix',
  people: [], eventIds: [], genres: [], styles: [], assets: [], sources: []
};

function metadata(facts: Record<string, unknown>, status: ProviderMetadata['match']['status'] = 'confirmed'): ProviderMetadata {
  return {
    provider: { provider: 'youtube', resourceType: 'video', externalId: 'video-30' },
    sourceUrl: 'https://www.youtube.com/watch?v=video-30',
    observedAt: '2026-10-03T12:00:00.000Z',
    match: { status, explanation: 'Video identity and duration match the recording' },
    facts
  };
}

test('provider adapters emit supported claims and do not promote upload or channel facts', () => {
  const claims = claimsFromProvider(target, metadata({
    description: 'Recording description',
    recordingDate: { value: '2024', precision: 'year' },
    uploadDate: { value: '2025-02-03', precision: 'day' },
    uploader: 'Archive Channel'
  }));
  assert.deepEqual(claims.map((claim) => claim.field), ['description', 'recordingDate']);
  assert.ok(claims.every((claim) => claim.evidence === 'direct'));
});

test('ambiguous source matches become parsed claims and cannot change the selected cover role', () => {
  const claims = claimsFromProvider(target, metadata({
    description: 'Possible match',
    cover: { role: 'mix-cover', url: 'https://i.ytimg.com/vi/video-30/hqdefault.jpg' }
  }, 'possible'));
  assert.ok(claims.every((claim) => claim.evidence === 'parsed'));
  assert.throws(() => claimsFromProvider(target, metadata({
    cover: { role: 'artist-portrait', url: 'https://example.org/avatar.jpg' }
  })));
});