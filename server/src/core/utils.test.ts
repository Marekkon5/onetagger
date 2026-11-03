import test from 'node:test';
import assert from 'node:assert/strict';
import { sanitizeProviderSnapshot, confidenceScore, duplicateKey, normalizeQuery, overlapScore, uniqueCandidates } from './utils.js';
import type { MixCandidate } from '../domain.js';

test('normalizeQuery strips technical noise but preserves identity tokens', () => {
  // File format is noise; "Live" is a meaningful part of the recording title.
  assert.equal(normalizeQuery('Gotek_-_Live @ NBT Party Roma 03-03-07.mp3'), 'gotek live @ nbt party roma 03 03 07');
});

test('normalizeQuery preserves meaningful artist and title tokens', () => {
  // Identity rule 3: names such as "DJ ..." or "Live ..." must not lose words.
  assert.equal(normalizeQuery('DJ Koalisson III'), 'dj koalisson iii');
  assert.equal(normalizeQuery('Live at Wacken 2003'), 'live at wacken 2003');
  assert.equal(normalizeQuery('Spiral Tribe live set'), 'spiral tribe live set');
});

test('normalizeQuery drops artifact words only in trailing position', () => {
  assert.equal(normalizeQuery('Aniane Live 98 Radio Show Mix'), 'aniane live 98 radio show');
  // "DJ set" and "live set" are meaningful descriptors, not file-format noise.
  assert.equal(normalizeQuery('Metek DJ set'), 'metek dj set');
  assert.equal(normalizeQuery('Spiral Tribe live set'), 'spiral tribe live set');
  // A title that consists only of an artifact word is not emptied out.
  assert.equal(normalizeQuery('Mix'), 'mix');
});

test('overlapScore rewards shared artist/event tokens', () => {
  assert.ok(overlapScore('Spiral Tribe warehouse 2001', 'Spiral Tribe London warehouse') > 0.6);
});

test('confidenceScore explains title and artist evidence', () => {
  const result = confidenceScore({ query: 'Gotek NBT Roma 2007', title: 'Live @ NBT Party Roma 03.03.07', artist: 'Gotek', pathContext: '/gotek/' });
  assert.ok(result.score > 0.5);
  assert.ok(result.reasons.length >= 1);
});

test('duplicateKey tolerates small duration variance in 30 second buckets', () => {
  const a = duplicateKey({ title: 'Aniane Live 98', artists: ['Metek'], durationMs: 5_390_000 });
  const b = duplicateKey({ title: 'Aniane Live 98', artists: ['METEK'], durationMs: 5_399_000 });
  assert.equal(a, b);
});

test('uniqueCandidates keeps the strongest duplicate', () => {
  const base: MixCandidate = {
    provider: 'freeteknomusic',
    title: 'Aniane Live 98',
    artists: ['Metek'],
    crews: [],
    durationMs: 5_390_000,
    source: { provider: 'freeteknomusic', url: 'https://example.test/a' },
    confidence: 0.6,
    reasons: [],
    raw: {},
  };
  const result = uniqueCandidates([base, { ...base, confidence: 0.92, source: { ...base.source, url: 'https://example.test/b' } }]);
  assert.equal(result.length, 1);
  assert.equal(result[0].confidence, 0.92);
});

test('provider snapshots remove credential-bearing keys and URL queries and bound recursive payloads',()=>{
 const result=sanitizeProviderSnapshot({authorization:'Bearer hidden',client_secret:'hidden',nested:{token:'hidden',url:'https://api.example.org/item?key=hidden&safe=yes'},long:'x'.repeat(5000),items:Array.from({length:100},(_,id)=>id)}) as any;
 assert.equal(result.authorization,undefined);assert.equal(result.client_secret,undefined);assert.equal(result.nested.token,undefined);assert.doesNotMatch(result.nested.url,/hidden|key=/);assert.equal(result.items.length,50);assert.equal(result.long.length,2000);
});
