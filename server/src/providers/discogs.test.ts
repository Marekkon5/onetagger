import assert from 'node:assert/strict';
import test from 'node:test';
import { DiscogsEnricher } from './discogs.js';

test('Discogs hydrates a unique exact artist from its entity endpoint', async (context) => {
  const calls: string[] = [];
  context.mock.method(globalThis, 'fetch', async (input: string | URL | Request) => {
    const url = String(input);
    calls.push(url);
    if (url.includes('database/search')) return Response.json({ results: [
      { id: 24216, title: 'Metek', type: 'artist' },
      { id: 491862, title: 'Metek (2)', type: 'artist' },
    ] });
    return Response.json({
      id: 24216,
      uri: 'https://www.discogs.com/artist/24216-Metek',
      profile: 'Real artist biography',
      images: [{ uri: 'https://i.discogs.com/metek.jpg' }],
    });
  });
  const result = await new DiscogsEnricher().enrichEntity('Metek', 'artist');
  assert.equal(calls.length, 2);
  assert.equal(result[0].profile, 'Real artist biography');
  assert.equal(result[0].url, 'https://www.discogs.com/artist/24216-Metek');
  assert.equal(result[1].profile, undefined);
});

test('Discogs hydrates an already-linked entity by resource ID without searching by name', async (context) => {
  const calls: string[] = [];
  context.mock.method(globalThis, 'fetch', async (input: string | URL | Request) => {
    calls.push(String(input));
    return Response.json({ id: 42, name: 'Mackitek', uri: 'https://www.discogs.com/artist/42-Mackitek', profile: 'Shared profile', images: [{ uri: 'https://i.discogs.com/artist.jpg' }] });
  });
  const entity = await new DiscogsEnricher().hydrateEntity('42', 'artist');
  assert.equal(calls.length, 1);
  assert.match(calls[0], /artists\/42$/);
  assert.equal(entity?.profile, 'Shared profile');
  assert.equal(entity?.imageUrl, 'https://i.discogs.com/artist.jpg');
});
